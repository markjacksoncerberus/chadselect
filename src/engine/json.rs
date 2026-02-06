//! JMESPath extraction engine.
//!
//! Processes JMESPath expressions against JSON content with lazy caching
//! of the parsed `serde_json::Value`.

use log::warn;

use crate::content::ContentItem;

/// Process a JMESPath expression against a content item, returning matches.
///
/// The parsed JSON value is cached on the [`ContentItem`] for reuse across
/// multiple queries against the same document.
pub fn process(path: &str, content_item: &ContentItem) -> Vec<String> {
    let mut json_value_ref = content_item.json_value.borrow_mut();

    if json_value_ref.is_none() {
        match serde_json::from_str(&content_item.content) {
            Ok(value) => {
                *json_value_ref = Some(value);
            }
            Err(e) => {
                warn!("Failed to parse JSON content: {}", e);
                return vec![];
            }
        }
    }

    let Some(json_value) = json_value_ref.as_ref() else {
        warn!("Failed to access cached JSON value");
        return vec![];
    };

    let expression = match jmespath::compile(path) {
        Ok(expr) => expr,
        Err(e) => {
            warn!("Invalid JMESPath expression '{}': {}", path, e);
            return vec![];
        }
    };

    let result = match expression.search(json_value) {
        Ok(value) => value,
        Err(e) => {
            warn!("JMESPath execution failed: {}", e);
            return vec![];
        }
    };

    jmespath_value_to_strings(&result)
}

/// Recursively convert a JMESPath result into a flat `Vec<String>`.
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
