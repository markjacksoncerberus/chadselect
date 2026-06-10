//! XPath 1.0 extraction engine.
//!
//! Evaluates XPath expressions over the **shared** `scraper`/html5ever document
//! (the same DOM the CSS engine uses) via `chadpath` and the
//! [`ENode`](crate::engine::xnode::ENode) adapter. The previous `sxd_html`
//! backend was replaced because its tree-build was quadratic in memory and time
//! on real-world and adversarial HTML.

use std::rc::Rc;

use log::warn;
use scraper::Html;

use crate::content::ContentItem;
use crate::engine::{xpath_eval, xpath_rewrite};
use crate::functions;

/// Inline-parse threshold for *nesting depth*. chadpath's recursive-descent parser
/// uses stack proportional to both nesting depth and overall expression
/// length/structure, and debug frames are several× fatter than release — so the
/// inline budget is tighter in debug builds. Anything above goes to a
/// large-stack thread.
const INLINE_SAFE_DEPTH: usize = if cfg!(debug_assertions) { 4 } else { 8 };

/// Inline-parse threshold for raw expression *length*. Long-but-shallow
/// selectors still recurse deeply in chadpath (the parser recurses per step /
/// operator, not just per bracket), so length is a second, independent trigger.
const INLINE_SAFE_LEN: usize = if cfg!(debug_assertions) { 96 } else { 384 };

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

    // Route to a large-stack thread when the expression is deeply nested OR
    // long enough that chadpath's recursion could overflow the caller's stack.
    let needs_deep_stack = depth > INLINE_SAFE_DEPTH || raw_expr.len() > INLINE_SAFE_LEN;

    let mut results = if depth > MAX_QUERY_DEPTH {
        warn!(
            "XPath expression nested {depth} levels deep (> {MAX_QUERY_DEPTH}); refusing to \
             avoid a stack overflow in the parser"
        );
        vec![]
    } else if needs_deep_stack {
        evaluate_on_deep_stack(&content_item.content, raw_expr)
    } else {
        // Positional predicates are evaluated correctly by the (forked) chadpath
        // engine, so no expression rewriting is needed.
        xpath_eval::evaluate(&content_item.html(), raw_expr)
    };

    if !text_functions.is_empty() {
        results = functions::apply_text_functions(results, &text_functions);
    }

    results
}

/// Parse and evaluate a deeply-nested expression on a thread with a large stack
/// — chadpath's recursive parser would otherwise overflow small (e.g. 2 MiB tokio
/// worker) caller stacks.
///
/// chadpath's `Transform<ENode>` and the `ENode` handle are `!Send` (they hold an
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
            let doc = Rc::new(Html::parse_document(&content));
            xpath_eval::evaluate(&doc, &raw_expr)
        })
        .ok()
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}
