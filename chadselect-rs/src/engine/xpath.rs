//! XPath 1.0 extraction engine.
//!
//! Evaluates XPath expressions over the **shared** `scraper`/html5ever document
//! (the same DOM the CSS engine uses) via `xrust` and the
//! [`ENode`](crate::engine::xnode::ENode) adapter. The previous `sxd_html`
//! backend was replaced because its tree-build was quadratic in memory and time
//! on real-world and adversarial HTML.

use std::rc::Rc;

use log::warn;
use scraper::Html;

use crate::content::ContentItem;
use crate::engine::{xpath_eval, xpath_rewrite};
use crate::functions;

/// Below this nesting depth an expression is parsed inline on the caller's
/// stack (~130 KiB/level in xrust's recursive parser); 8 levels stays well
/// under a 2 MiB tokio-worker stack.
const INLINE_SAFE_DEPTH: usize = 8;

/// Deeper expressions are parsed on a thread with this stack so the recursive
/// parser cannot overflow small caller stacks (~4000 levels of headroom).
const DEEP_STACK_BYTES: usize = 512 * 1024 * 1024;

/// Expressions nested beyond this are refused outright (returning empty rather
/// than risking a stack overflow even on the large stack). No real selector
/// approaches this.
const MAX_QUERY_DEPTH: usize = 2000;

/// Process an XPath expression (potentially with a `>>` function chain)
/// against a content item, returning matches. Never panics.
pub fn process(xpath_with_functions: &str, content_item: &ContentItem) -> Vec<String> {
    let (raw_expr, text_functions) = functions::split_functions(xpath_with_functions);

    // Depth is computed iteratively (stack-safe) on the *raw* expression and
    // decides routing BEFORE the recursive rewrite/parser run, which would
    // themselves overflow a small stack on pathologically nested input.
    let depth = xpath_rewrite::nesting_depth(raw_expr);

    let mut results = if depth > MAX_QUERY_DEPTH {
        warn!(
            "XPath expression nested {depth} levels deep (> {MAX_QUERY_DEPTH}); refusing to \
             avoid a stack overflow in the parser"
        );
        vec![]
    } else if depth > INLINE_SAFE_DEPTH {
        evaluate_on_deep_stack(&content_item.content, raw_expr)
    } else {
        let expr = xpath_rewrite::rewrite_positional_predicates(raw_expr);
        xpath_eval::evaluate(&content_item.html(), &expr)
    };

    if !text_functions.is_empty() {
        results = functions::apply_text_functions(results, &text_functions);
    }

    results
}

/// Rewrite, parse, and evaluate a deeply-nested expression on a thread with a
/// large stack — both the rewriter and xrust's parser recurse per nesting
/// level and would overflow small (e.g. 2 MiB tokio worker) caller stacks.
///
/// xrust's `Transform<ENode>` and the `ENode` handle are `!Send` (they hold an
/// `Rc<Html>`), so the document is re-parsed inside the thread from the raw
/// content (`String` is `Send`). The extra parse only affects pathologically
/// deep expressions, which are rare.
fn evaluate_on_deep_stack(content: &str, raw_expr: &str) -> Vec<String> {
    let content = content.to_owned();
    let raw_expr = raw_expr.to_owned();
    std::thread::Builder::new()
        .name("chadselect-xpath".into())
        .stack_size(DEEP_STACK_BYTES)
        .spawn(move || {
            let expr = xpath_rewrite::rewrite_positional_predicates(&raw_expr);
            let doc = Rc::new(Html::parse_document(&content));
            xpath_eval::evaluate(&doc, &expr)
        })
        .ok()
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}
