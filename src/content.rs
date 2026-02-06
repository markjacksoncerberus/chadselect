//! Content types and content item storage with lazy-parsed caching.

use scraper::Html;
use serde_json::Value;
use sxd_document::Package;
use std::cell::RefCell;
use std::collections::HashMap;

/// Content type enumeration for explicit content specification.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    /// Plain text content — compatible with Regex and XPath.
    Text,
    /// HTML content — compatible with Regex, XPath, and CSS selectors.
    Html,
    /// JSON content — compatible with Regex and JMESPath.
    Json,
}

/// A single content item with its type and lazily-cached parsed representations.
///
/// Parsed documents are created on first access and reused for subsequent queries,
/// avoiding redundant parsing when multiple queries target the same content.
#[derive(Debug)]
pub struct ContentItem {
    /// Raw content string.
    pub content: String,
    /// Declared content type.
    pub content_type: ContentType,
    /// Lazily parsed XPath document (via `sxd_html`).
    pub(crate) xpath_document: RefCell<Option<Package>>,
    /// Lazily parsed JSON value.
    pub(crate) json_value: RefCell<Option<Value>>,
    /// Lazily parsed HTML document for CSS selectors (via `scraper`).
    pub(crate) css_document: RefCell<Option<Html>>,
    /// Element-text cache for CSS text pseudo-selectors: selector → Vec<(element_index, text)>.
    pub(crate) element_text_cache: RefCell<HashMap<String, Vec<(usize, String)>>>,
}

impl ContentItem {
    /// Create a new content item with the given content and type.
    pub fn new(content: String, content_type: ContentType) -> Self {
        Self {
            content,
            content_type,
            xpath_document: RefCell::new(None),
            json_value: RefCell::new(None),
            css_document: RefCell::new(None),
            element_text_cache: RefCell::new(HashMap::new()),
        }
    }
}

impl Clone for ContentItem {
    fn clone(&self) -> Self {
        // Don't clone cached documents — they will be lazily re-parsed if needed.
        ContentItem::new(self.content.clone(), self.content_type.clone())
    }
}
