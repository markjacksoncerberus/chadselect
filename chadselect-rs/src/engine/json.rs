//! JMESPath extraction engine.
//!
//! Processes JMESPath expressions against JSON content. The document is parsed
//! into a JMESPath value tree (`Rc<Variable>`) once and cached per document
//! (see [`ContentItem::jmespath_value`]); compiled expressions are cached per
//! thread. Every query then evaluates against the cached tree without
//! re-converting the document.

use std::cell::RefCell;
use std::collections::HashMap;

use jmespath::Expression;
use log::warn;

use crate::content::ContentItem;

thread_local! {
    /// Cache of compiled JMESPath expressions, keyed by the expression string.
    ///
    /// `jmespath::compile` parses the expression on every call; the converted
    /// document tree is already cached per document, but the *expression* was
    /// recompiled per query (a flat slice of fleet CPU). The crawler runs the
    /// same expressions across many documents, so compiling once per distinct
    /// expression (per thread) removes that. `Expression<'static>` is `Clone`
    /// (cheap — it shares the parsed AST), so we clone out of the cache. Invalid
    /// expressions cache as `None` to avoid re-parsing / re-warning each call.
    static COMPILED: RefCell<HashMap<String, Option<Expression<'static>>>> =
        RefCell::new(HashMap::new());
}

/// Compile `path` (or fetch the cached `Expression`). Returns `None` for an
/// invalid expression, warning once on first compile.
fn compiled(path: &str) -> Option<Expression<'static>> {
    COMPILED.with(|c| {
        if let Some(e) = c.borrow().get(path) {
            return e.clone();
        }
        let compiled = match jmespath::compile(path) {
            Ok(expr) => Some(expr),
            Err(e) => {
                warn!("Invalid JMESPath expression '{}': {}", path, e);
                None
            }
        };
        c.borrow_mut().insert(path.to_string(), compiled.clone());
        compiled
    })
}

/// Process a JMESPath expression against a content item, returning matches.
///
/// The document is parsed into a JMESPath value tree (`Rc<Variable>`) **once**
/// and cached on the [`ContentItem`]; every query then evaluates against the
/// cached tree via [`Expression::search_cached`]. This is the key allocation
/// win: upstream `Expression::search` re-converts the whole document into a
/// fresh `Rc<Variable>` tree on *every* call (a full serde walk — the dominant
/// allocation source on JSON-heavy pages). Caching the converted tree collapses
/// that to a single conversion per document, shared by reference across queries.
pub fn process(path: &str, content_item: &ContentItem) -> Vec<String> {
    let mut jmespath_ref = content_item.jmespath_value.borrow_mut();

    if jmespath_ref.is_none() {
        // Parse the raw content straight into the JMESPath tree (one serde pass),
        // skipping the intermediate `serde_json::Value` entirely.
        match jmespath::Variable::from_json(&content_item.content) {
            Ok(var) => *jmespath_ref = Some(jmespath::Rcvar::new(var)),
            Err(e) => {
                warn!("Failed to parse JSON content: {}", e);
                return vec![];
            }
        }
    }

    let Some(data) = jmespath_ref.as_ref() else {
        warn!("Failed to access cached JMESPath value");
        return vec![];
    };

    let expression = match compiled(path) {
        Some(expr) => expr,
        None => return vec![],
    };

    let result = match expression.search_cached(data) {
        Ok(value) => value,
        Err(e) => {
            warn!("JMESPath execution failed: {}", e);
            return vec![];
        }
    };

    jmespath_value_to_strings(&result)
}

/// Recursively convert a JMESPath result into a flat `Vec<String>`.
#[cfg(test)]
mod cache_tests {
    use super::*;

    /// Guard: JMESPath expressions compile once and are cached per distinct
    /// expression. Fails (or stops compiling) if the compile cache is removed.
    #[test]
    fn expressions_are_compiled_once_and_cached() {
        COMPILED.with(|c| c.borrow_mut().clear());
        let _ = compiled("foo.bar");
        let _ = compiled("foo.bar");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 1, "repeated expr must compile once");
        let _ = compiled("baz[0]");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 2, "distinct expr adds one entry");
    }
}

fn jmespath_value_to_strings(value: &jmespath::Variable) -> Vec<String> {
    match value {
        jmespath::Variable::Null => vec![],
        jmespath::Variable::Bool(b) => vec![b.to_string()],
        jmespath::Variable::Number(n) => vec![n.to_string()],
        jmespath::Variable::String(s) => vec![s.clone()],
        jmespath::Variable::Array(arr) => {
            arr.iter().flat_map(|item| jmespath_value_to_strings(item)).collect()
        }
        jmespath::Variable::Object(obj) => {
            let mut json_parts = Vec::new();
            json_parts.push("{".to_string());
            for (i, (key, value)) in obj.iter().enumerate() {
                if i > 0 {
                    json_parts.push(",".to_string());
                }
                let value_strings = jmespath_value_to_strings(value);
                let value_str = if value_strings.len() == 1 {
                    value_strings[0].clone()
                } else {
                    format!("[{}]", value_strings.join(","))
                };
                json_parts.push(format!(
                    "\"{}\":{}",
                    key,
                    if matches!(&**value, jmespath::Variable::String(_)) {
                        format!("\"{}\"", value_str)
                    } else {
                        value_str
                    }
                ));
            }
            json_parts.push("}".to_string());
            vec![json_parts.join("")]
        }
        jmespath::Variable::Expref(_) => {
            vec!["<expression>".to_string()]
        }
    }
}
