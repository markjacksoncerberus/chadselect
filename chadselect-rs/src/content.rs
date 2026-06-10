//! Content types and content item storage with lazy-parsed caching.

use scraper::Html;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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
    /// Lazily parsed JSON value.
    pub(crate) json_value: RefCell<Option<Value>>,
    /// Lazily parsed HTML document (via `scraper`/html5ever), **shared** by
    /// both the CSS and XPath engines — the HTML is parsed exactly once.
    pub(crate) html_document: RefCell<Option<Rc<Html>>>,
    /// Element-text cache for CSS text pseudo-selectors: selector → Vec<(element_index, text)>.
    pub(crate) element_text_cache: RefCell<HashMap<String, Vec<(usize, String)>>>,
}

impl ContentItem {
    /// Create a new content item with the given content and type.
    pub fn new(content: String, content_type: ContentType) -> Self {
        Self {
            content,
            content_type,
            json_value: RefCell::new(None),
            html_document: RefCell::new(None),
            element_text_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Get the shared, lazily-parsed HTML document, parsing it on first use.
    ///
    /// Both the CSS engine (`scraper` selectors) and the XPath engine (via the
    /// `chadpath` adapter) call this, so a document is parsed by html5ever **once**
    /// regardless of how many or which kinds of queries run against it.
    pub(crate) fn html(&self) -> Rc<Html> {
        let mut doc = self.html_document.borrow_mut();
        if doc.is_none() {
            *doc = Some(Rc::new(Html::parse_document(&self.content)));
        }
        doc.as_ref().unwrap().clone()
    }
}

impl Clone for ContentItem {
    fn clone(&self) -> Self {
        // Don't clone cached documents — they will be lazily re-parsed if needed.
        ContentItem::new(self.content.clone(), self.content_type.clone())
    }
}
