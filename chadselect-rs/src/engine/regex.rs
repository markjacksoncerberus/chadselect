//! Regex extraction engine.
//!
//! Processes regex patterns against any content type. Capture groups are
//! extracted automatically — if the pattern contains groups, only group
//! values are returned; otherwise full matches are returned.

use log::warn;
use regex::Regex;

use crate::content::ContentType;

/// Process a regex pattern against content, returning all matches.
///
/// - If the regex contains capture groups, captured values are returned.
/// - If no capture groups, full match strings are returned.
/// - Invalid patterns return an empty vector (never panics).
pub fn process(pattern: &str, content: &str, _content_type: &ContentType) -> Vec<String> {
    let regex = match Regex::new(pattern) {
        Ok(r) => r,
        Err(e) => {
            warn!("Invalid regex pattern '{}': {}", pattern, e);
            return vec![];
        }
    };

    let mut results = Vec::new();

    if regex.captures_len() > 1 {
        // Has capture groups — extract group values
        for capture in regex.captures_iter(content) {
            for i in 1..capture.len() {
                if let Some(matched) = capture.get(i) {
                    results.push(matched.as_str().to_string());
                }
            }
        }
    } else {
        // No capture groups — return full matches
        for mat in regex.find_iter(content) {
            results.push(mat.as_str().to_string());
        }
    }

    results
}
