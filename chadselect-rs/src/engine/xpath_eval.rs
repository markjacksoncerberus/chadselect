//! XPath 1.0 evaluation over the shared `scraper` DOM, via `xrust` and the
//! [`ENode`](crate::engine::xnode::ENode) adapter. No second parse occurs — the
//! same html5ever `ego-tree` the CSS engine builds is queried directly.

use std::rc::Rc;

use scraper::Html;
use xrust::item::{Item, Node};
use xrust::parser::xpath::parse;
use xrust::transform::context::{ContextBuilder, StaticContextBuilder};
use xrust::xdmerror::{Error, ErrorKind};

use crate::engine::xnode::ENode;

/// Evaluate `expr` over an already-parsed `Html` document, returning trimmed,
/// non-empty string values in document order. Never panics; invalid
/// expressions or evaluation errors yield an empty vector.
pub fn evaluate(doc: &Rc<Html>, expr: &str) -> Vec<String> {
    let mut stctxt = StaticContextBuilder::new()
        .message(|_| Ok(()))
        .fetcher(|_| Err(Error::new(ErrorKind::NotImplemented, "no fetcher")))
        .parser(|_| Err(Error::new(ErrorKind::NotImplemented, "no parser")))
        .build();

    let transform = match parse::<ENode>(expr, None, None) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let ctxt = ContextBuilder::new()
        .context(vec![Item::Node(ENode::root_of(doc))])
        .result_document(ENode::new_document())
        .build();

    match ctxt.dispatch(&mut stctxt, &transform) {
        Ok(seq) => seq
            .iter()
            .map(|item| item.to_string().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => vec![],
    }
}
