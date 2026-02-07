//! Query type parsing — prefix-based routing to the correct extraction engine.

use crate::content::ContentType;

/// The function-pipe delimiter used to separate a selector expression from its
/// post-processing function chain.
///
/// We use `>>` instead of `|` because `|` is a union operator in XPath 1.0
/// and a pipe operator in JMESPath, which would create ambiguity.
///
/// ```text
/// css:.price >> normalize-space() >> uppercase()
/// xpath://div[@class='vin']/text() >> substring-after('VIN: ')
/// ```
pub const FUNCTION_PIPE: &str = ">>";

/// Parsed query type with the engine-specific expression.
#[derive(Debug, Clone)]
pub enum QueryType {
    /// Regex pattern — works on all content types.
    Regex(String),
    /// XPath 1.0 expression — works on HTML and Text.
    XPath(String),
    /// JMESPath expression — works on JSON.
    JsonPath(String),
    /// CSS selector — works on HTML.
    CssSelector(String),
}

/// Parse a prefixed query string into its typed representation.
///
/// Supported prefixes:
/// - `regex:` → [`QueryType::Regex`]
/// - `xpath:` → [`QueryType::XPath`]
/// - `json:`  → [`QueryType::JsonPath`]
/// - `css:`   → [`QueryType::CssSelector`]
///
/// If no prefix is provided, the query defaults to Regex.
pub fn parse_query(query: &str) -> Result<QueryType, String> {
    if let Some(pattern) = query.strip_prefix("regex:") {
        Ok(QueryType::Regex(pattern.to_string()))
    } else if let Some(path) = query.strip_prefix("json:") {
        Ok(QueryType::JsonPath(path.to_string()))
    } else if let Some(xpath) = query.strip_prefix("xpath:") {
        Ok(QueryType::XPath(xpath.to_string()))
    } else if let Some(selector) = query.strip_prefix("css:") {
        Ok(QueryType::CssSelector(selector.to_string()))
    } else {
        // Default to regex if no prefix is provided
        Ok(QueryType::Regex(query.to_string()))
    }
}

/// Check whether a query type is compatible with a content type.
pub fn is_query_compatible(query_type: &QueryType, content_type: &ContentType) -> bool {
    match query_type {
        QueryType::Regex(_) => true,
        QueryType::JsonPath(_) => matches!(content_type, ContentType::Json),
        QueryType::CssSelector(_) => matches!(content_type, ContentType::Html),
        QueryType::XPath(_) => matches!(content_type, ContentType::Html | ContentType::Text),
    }
}
