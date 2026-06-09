//! XPath 1.0 extraction engine.
//!
//! Evaluates XPath expressions over the **shared** `scraper`/html5ever document
//! (the same DOM the CSS engine uses) via `xrust` and the
//! [`ENode`](crate::engine::xnode::ENode) adapter. The previous `sxd_html`
//! backend was replaced because its tree-build was quadratic in memory and time
//! on real-world and adversarial HTML.

use crate::content::ContentItem;
use crate::engine::xpath_eval;
use crate::functions;

/// Process an XPath expression (potentially with a `>>` function chain)
/// against a content item, returning matches. Never panics.
pub fn process(xpath_with_functions: &str, content_item: &ContentItem) -> Vec<String> {
    let (xpath_expr, text_functions) = functions::split_functions(xpath_with_functions);

    let document = content_item.html();
    let mut results = xpath_eval::evaluate(&document, xpath_expr);

    if !text_functions.is_empty() {
        results = functions::apply_text_functions(results, &text_functions);
    }

    results
}
