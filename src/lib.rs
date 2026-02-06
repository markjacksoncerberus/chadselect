//! # ChadSelect
//!
//! Unified data extraction — **Regex**, **XPath 1.0**, **CSS Selectors**, and
//! **JMESPath** behind one query interface.
//!
//! ```rust
//! use chadselect::ChadSelect;
//!
//! let mut cs = ChadSelect::new();
//! cs.add_html(r#"<span class="price">$49.99</span>"#.to_string());
//!
//! let price = cs.select(0, "css:.price");
//! assert_eq!(price, "$49.99");
//! ```
//!
//! ## Query prefixes
//!
//! | Prefix   | Engine    | Content types |
//! |----------|-----------|---------------|
//! | `regex:` | Regex     | All           |
//! | `xpath:` | XPath 1.0 | HTML, Text    |
//! | `css:`   | CSS       | HTML          |
//! | `json:`  | JMESPath  | JSON          |
//!
//! If no prefix is provided, the query defaults to Regex.
//!
//! ## Post-processing functions
//!
//! Pipe results through text functions using `>>`:
//!
//! ```text
//! css:.price >> normalize-space() >> uppercase()
//! xpath://div/text() >> substring-after('VIN: ')
//! ```

pub mod content;
pub mod engine;
pub mod functions;
pub mod query;

use std::collections::HashSet;

use log::warn;

pub use content::{ContentItem, ContentType};
pub use functions::supported_text_functions;
pub use query::{QueryType, FUNCTION_PIPE};

use engine::xpath::XPathCache;

/// Main entry point for data extraction.
///
/// Load content via [`add_html`](ChadSelect::add_html),
/// [`add_json`](ChadSelect::add_json), or [`add_text`](ChadSelect::add_text),
/// then query with [`select`](ChadSelect::select) or
/// [`query`](ChadSelect::query).
pub struct ChadSelect {
    content_list: Vec<ContentItem>,
    xpath_cache: XPathCache,
}

impl ChadSelect {
    /// Create a new, empty `ChadSelect` instance.
    pub fn new() -> Self {
        Self {
            content_list: Vec::new(),
            xpath_cache: XPathCache::new(),
        }
    }

    // ── Content management ──────────────────────────────────────────────

    /// Add plain text content.
    pub fn add_text(&mut self, content: String) {
        self.content_list
            .push(ContentItem::new(content, ContentType::Text));
    }

    /// Add HTML content (compatible with CSS, XPath, and Regex).
    pub fn add_html(&mut self, content: String) {
        self.content_list
            .push(ContentItem::new(content, ContentType::Html));
    }

    /// Add JSON content (compatible with JMESPath and Regex).
    pub fn add_json(&mut self, content: String) {
        self.content_list
            .push(ContentItem::new(content, ContentType::Json));
    }

    /// Return the number of loaded content items.
    pub fn content_count(&self) -> usize {
        self.content_list.len()
    }

    /// Remove all loaded content.
    pub fn clear(&mut self) {
        self.content_list.clear();
    }

    // ── Querying ────────────────────────────────────────────────────────

    /// Query all loaded content and return matching results.
    ///
    /// - `index = -1` returns **all** matches.
    /// - `index >= 0` returns the match at that position (or empty if out of
    ///   bounds).
    ///
    /// **Never panics** — invalid queries or out-of-bounds indices return an
    /// empty vector.
    pub fn query(
        &self,
        index: i32,
        query_str: &str,
    ) -> Vec<String> {
        let query_type = match query::parse_query(query_str) {
            Ok(qt) => qt,
            Err(_) => {
                warn!("Failed to parse query: {}", query_str);
                return vec![];
            }
        };

        let mut all_results = Vec::new();

        for content_item in &self.content_list {
            if !query::is_query_compatible(&query_type, &content_item.content_type) {
                continue;
            }

            let results = match &query_type {
                QueryType::Regex(pattern) => {
                    engine::regex::process(pattern, &content_item.content, &content_item.content_type)
                }
                QueryType::JsonPath(path) => engine::json::process(path, content_item),
                QueryType::CssSelector(selector) => engine::css::process(selector, content_item),
                QueryType::XPath(xpath) => {
                    engine::xpath::process(xpath, content_item, &self.xpath_cache)
                }
            };

            all_results.extend(results);
        }

        select_by_index(all_results, index)
    }

    /// Return a single result string (the first match), or an empty string.
    ///
    /// A result is considered **valid** when it is not empty and not
    /// whitespace-only.
    ///
    /// Shorthand for `query(index, query_str)[0]` with safe fallback.
    pub fn select(&self, index: i32, query_str: &str) -> String {
        self.select_where(index, query_str, default_valid)
    }

    /// Like [`select`](ChadSelect::select) but with a custom validity check.
    ///
    /// The `valid` closure receives each candidate `&str` and returns `true`
    /// if the value should be accepted.
    ///
    /// ```rust
    /// use chadselect::ChadSelect;
    ///
    /// let mut cs = ChadSelect::new();
    /// cs.add_text("count: 0".to_string());
    ///
    /// // Default: "0" is valid (non-empty, non-whitespace)
    /// let r = cs.select(0, r"(\d+)");
    /// assert_eq!(r, "0");
    ///
    /// // Custom: reject "0" as invalid
    /// let r = cs.select_where(0, r"(\d+)", |s| s != "0");
    /// assert_eq!(r, "");
    /// ```
    pub fn select_where<F>(&self, index: i32, query_str: &str, valid: F) -> String
    where
        F: Fn(&str) -> bool,
    {
        let result = self.query(index, query_str);
        if !result.is_empty() && valid(&result[0]) {
            return result[0].clone();
        }
        String::new()
    }

    /// Try multiple queries in order and return the first valid result set.
    ///
    /// A result is considered **valid** when it is not empty and not
    /// whitespace-only.
    ///
    /// Useful for fallback chains where several selectors may match the data.
    pub fn select_first(&self, queries: Vec<(i32, &str)>) -> Vec<String> {
        self.select_first_where(queries, default_valid)
    }

    /// Like [`select_first`](ChadSelect::select_first) but with a custom
    /// validity check.
    ///
    /// The `valid` closure receives each candidate `&str` and returns `true`
    /// if the value should be accepted. The first query whose results
    /// **all** pass the check wins.
    ///
    /// ```rust
    /// use chadselect::ChadSelect;
    ///
    /// let mut cs = ChadSelect::new();
    /// cs.add_text("a: 0\nb: 42".to_string());
    ///
    /// // First query matches "0", but our validator rejects it.
    /// // Falls through to the second query which matches "42".
    /// let r = cs.select_first_where(
    ///     vec![(0, r"a: (\d+)"), (0, r"b: (\d+)")],
    ///     |s| s != "0",
    /// );
    /// assert_eq!(r, vec!["42"]);
    /// ```
    pub fn select_first_where<F>(&self, queries: Vec<(i32, &str)>, valid: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        for (index, query_str) in queries {
            let result = self.query(index, query_str);
            if !result.is_empty() && result.iter().all(|r| valid(r)) {
                return result;
            }
        }
        vec![]
    }

    /// Run multiple queries and return the combined unique results.
    ///
    /// Only results that are non-empty and non-whitespace are included.
    pub fn select_many(&self, queries: Vec<(i32, &str)>) -> Vec<String> {
        self.select_many_where(queries, default_valid)
    }

    /// Like [`select_many`](ChadSelect::select_many) but with a custom
    /// validity check.
    ///
    /// The `valid` closure receives each candidate `&str` and returns `true`
    /// if the value should be included in the output.
    pub fn select_many_where<F>(&self, queries: Vec<(i32, &str)>, valid: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        let mut all_results = HashSet::new();
        for (index, query_str) in queries {
            let results = self.query(index, query_str);
            for result in results {
                if valid(&result) {
                    all_results.insert(result);
                }
            }
        }
        all_results.into_iter().collect()
    }
}

impl Default for ChadSelect {
    fn default() -> Self {
        Self::new()
    }
}

/// Select results by index — `-1` means "all".
fn select_by_index(results: Vec<String>, index: i32) -> Vec<String> {
    match index {
        -1 => results,
        i if i >= 0 => match results.get(i as usize) {
            Some(result) => vec![result.clone()],
            None => {
                warn!(
                    "Index {} out of range (have {} results)",
                    i,
                    results.len()
                );
                vec![]
            }
        },
        _ => {
            warn!("Invalid index: {}", index);
            vec![]
        }
    }
}

/// Default validity check used by [`ChadSelect::select`],
/// [`ChadSelect::select_first`], and [`ChadSelect::select_many`].
///
/// A value is valid when it is **not empty** and **not whitespace-only**.
pub fn default_valid(value: &str) -> bool {
    !value.trim().is_empty()
}
