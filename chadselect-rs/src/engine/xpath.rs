//! XPath 1.0 extraction engine.
//!
//! Evaluates XPath expressions over the **shared** `scraper`/html5ever document
//! (the same DOM the CSS engine uses) via `chadpath` and the
//! [`ENode`](crate::engine::xnode::ENode) adapter. The previous `sxd_html`
//! backend was replaced because its tree-build was quadratic in memory and time
//! on real-world and adversarial HTML.

use log::warn;

use crate::content::ContentItem;
use crate::engine::{xpath_eval, xpath_rewrite};
use crate::functions;

/// Maximum `(`/`[` nesting we hand to chadpath's parser. Beyond this we refuse
/// the expression (warn + empty) rather than risk a stack overflow, because
/// chadpath's parser-combinators re-descend the whole expression grammar per
/// nesting level — a stack-hungry recursion.
///
/// This bound is the *only* gate, and it is checked iteratively (stack-safe,
/// see [`xpath_rewrite::nesting_depth`]) **before** the recursive parser runs.
/// It is set from measurement, not guesswork (examples/parser_depth_probe.rs):
///
/// | build / stack        | measured overflow depth |
/// |----------------------|-------------------------|
/// | release / 2 MiB tokio worker | ~34             |
/// | debug   / 2 MiB              | ~18             |
/// | release / 8 MiB main thread  | ~160            |
///
/// Real fleet selectors top out at nesting depth **7**. `16` therefore clears
/// the real workload ~2.3× over while staying ~2× under the tightest
/// (release / 2 MiB) overflow — and only un-nested length, which does *not*
/// drive recursion (path/operator repetition is iterative), is no longer
/// considered at all.
const MAX_NESTING_DEPTH: usize = 16;

/// Process an XPath expression (potentially with a `>>` function chain)
/// against a content item, returning matches. Never panics.
pub fn process(xpath_with_functions: &str, content_item: &ContentItem) -> Vec<String> {
    let (raw_expr, text_functions) = functions::split_functions(xpath_with_functions);

    // Stack-safety gate: an iterative (non-recursive) depth scan decides whether
    // it's safe to hand the expression to chadpath's recursive-descent parser.
    // Pathologically nested expressions are refused here rather than risking a
    // process-killing stack overflow inside the parser.
    let depth = xpath_rewrite::nesting_depth(raw_expr);

    let mut results = if depth > MAX_NESTING_DEPTH {
        warn!(
            "XPath expression nested {depth} levels deep (> {MAX_NESTING_DEPTH}); refusing to \
             avoid a stack overflow in chadpath's recursive parser"
        );
        vec![]
    } else {
        // Inline on the shared, cached parsed document + cached document-order
        // map — no reparse, no per-query order rebuild. Positional predicates are
        // evaluated correctly by the (forked) chadpath engine, so no rewriting
        // is needed.
        let (doc, order) = content_item.html_with_order();
        xpath_eval::evaluate_with_order(&doc, order, raw_expr)
    };

    if !text_functions.is_empty() {
        results = functions::apply_text_functions(results, &text_functions);
    }

    results
}
