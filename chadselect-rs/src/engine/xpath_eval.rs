//! XPath 1.0 evaluation over the shared `scraper` DOM, via `chadpath` and the
//! [`ENode`](crate::engine::xnode::ENode) adapter. No second parse occurs — the
//! same html5ever `ego-tree` the CSS engine builds is queried directly.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use scraper::Html;
use chadpath::item::{Item, Node};
use chadpath::parser::xpath::parse;
use chadpath::transform::context::{ContextBuilder, StaticContextBuilder};
use chadpath::transform::Transform;
use chadpath::xdmerror::{Error, ErrorKind};

use crate::engine::xnode::ENode;

thread_local! {
    /// Cache of compiled XPath expressions, keyed by the expression string.
    ///
    /// chadpath's recursive-descent parser is the dominant cost (~5× evaluation),
    /// and a crawler runs the same handful of selectors across many documents.
    /// Compiling once per distinct expression (per thread) turns that into a
    /// one-time cost. The compiled `Transform` is document-independent, so it is
    /// reused across documents.
    static COMPILED: RefCell<HashMap<String, Rc<Transform<ENode>>>> =
        RefCell::new(HashMap::new());
}

/// Compile `expr` to a (cached) `Transform`. Returns `None` on a parse error.
fn compile(expr: &str) -> Option<Rc<Transform<ENode>>> {
    COMPILED.with(|c| {
        if let Some(t) = c.borrow().get(expr) {
            return Some(t.clone());
        }
        match parse::<ENode>(expr, None, None) {
            Ok(t) => {
                let rc = Rc::new(t);
                c.borrow_mut().insert(expr.to_string(), rc.clone());
                Some(rc)
            }
            Err(_) => None,
        }
    })
}

/// Evaluate `expr` over an already-parsed `Html` document, returning trimmed,
/// non-empty string values in document order. Never panics; invalid
/// expressions or evaluation errors yield an empty vector.
pub fn evaluate(doc: &Rc<Html>, expr: &str) -> Vec<String> {
    let Some(transform) = compile(expr) else {
        return vec![];
    };

    let mut stctxt = StaticContextBuilder::new()
        .message(|_| Ok(()))
        .fetcher(|_| Err(Error::new(ErrorKind::NotImplemented, "no fetcher")))
        .parser(|_| Err(Error::new(ErrorKind::NotImplemented, "no parser")))
        .build();

    let ctxt = ContextBuilder::new()
        .context(vec![Item::Node(ENode::root_of(doc))])
        .result_document(ENode::new_document())
        .build();

    match ctxt.dispatch(&mut stctxt, transform.as_ref()) {
        Ok(seq) => seq
            .iter()
            .map(|item| item.to_string().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => vec![],
    }
}
