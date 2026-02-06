//! XPath 1.0 extraction engine.
//!
//! Processes XPath expressions against HTML content using `sxd_html` for
//! HTML→document parsing and `sxd_xpath` for evaluation.  Documents and
//! XPath factories/contexts are lazily cached for performance.

use log::warn;
use sxd_xpath::{Value as XPathValue, Factory, Context, nodeset::Node};
use std::cell::RefCell;

use crate::content::ContentItem;
use crate::functions;

/// Cached XPath compilation state.
///
/// Stored on [`ChadSelect`](crate::ChadSelect) and threaded in to avoid
/// recreating the factory and context on every query.
pub struct XPathCache {
    pub factory: RefCell<Option<Factory>>,
    pub context: RefCell<Option<Context<'static>>>,
}

impl XPathCache {
    pub fn new() -> Self {
        Self {
            factory: RefCell::new(None),
            context: RefCell::new(None),
        }
    }
}

impl Default for XPathCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Process an XPath expression (potentially with `>>` function chain)
/// against a content item, returning matches.
pub fn process(
    xpath_with_functions: &str,
    content_item: &ContentItem,
    cache: &XPathCache,
) -> Vec<String> {
    let (xpath_expr, text_functions) = functions::split_functions(xpath_with_functions);

    // Ensure the document is parsed and cached.
    let mut xpath_doc_ref = content_item.xpath_document.borrow_mut();
    if xpath_doc_ref.is_none() {
        let package = sxd_html::parse_html(&content_item.content);
        *xpath_doc_ref = Some(package);
    }

    let Some(package) = xpath_doc_ref.as_ref() else {
        warn!("Failed to access cached XPath document");
        return vec![];
    };

    let document = package.as_document();

    // Ensure factory is created.
    let mut factory_ref = cache.factory.borrow_mut();
    if factory_ref.is_none() {
        *factory_ref = Some(Factory::new());
    }
    let factory = factory_ref.as_ref().unwrap();

    let expression = match factory.build(xpath_expr) {
        Ok(Some(expr)) => expr,
        Ok(None) => {
            warn!("XPath expression compiled to nothing: {}", xpath_expr);
            return vec![];
        }
        Err(e) => {
            warn!("XPath build failed for '{}': {:?}", xpath_expr, e);
            return vec![];
        }
    };

    // Ensure context is created.
    let mut context_ref = cache.context.borrow_mut();
    if context_ref.is_none() {
        *context_ref = Some(Context::new());
    }
    let context = context_ref.as_ref().unwrap();

    let xpath_result = match expression.evaluate(context, document.root()) {
        Ok(result) => result,
        Err(e) => {
            warn!("XPath execution failed: {:?}", e);
            return vec![];
        }
    };

    let mut results = xpath_value_to_strings(&xpath_result);

    if !text_functions.is_empty() {
        results = functions::apply_text_functions(results, &text_functions);
    }

    results
}

/// Convert an XPath evaluation result into a flat `Vec<String>`.
fn xpath_value_to_strings(result: &XPathValue) -> Vec<String> {
    match result {
        XPathValue::Nodeset(nodeset) => {
            let mut results = Vec::new();
            for node in nodeset.document_order() {
                let text_value = match node {
                    Node::Text(text_node) => text_node.text().to_string(),
                    Node::Element(_) => node.string_value(),
                    _ => node.string_value(),
                };
                let trimmed = text_value.trim();
                if !trimmed.is_empty() {
                    results.push(trimmed.to_string());
                }
            }
            results
        }
        XPathValue::String(s) => {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                vec![trimmed.to_string()]
            } else {
                vec![]
            }
        }
        XPathValue::Number(n) => vec![n.to_string()],
        XPathValue::Boolean(b) => vec![b.to_string()],
    }
}
