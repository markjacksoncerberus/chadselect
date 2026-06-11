//! Regex extraction engine.
//!
//! Processes regex patterns against any content type. Capture groups are
//! extracted automatically — if the pattern contains groups, only group
//! values are returned; otherwise full matches are returned.

use std::cell::RefCell;
use std::collections::HashMap;

use log::warn;
use regex::Regex;

use crate::content::ContentType;

thread_local! {
    /// Cache of compiled regexes, keyed by the pattern string.
    ///
    /// `Regex::new` is the dominant per-call cost (regex compilation —
    /// `build_many_from_hir` — measured ~28% of fleet CPU when recompiled per
    /// page), and a crawler runs the same handful of patterns across many
    /// documents. Compiling once per distinct pattern (per thread) makes it a
    /// one-time cost; `Regex` is internally `Arc`-backed, so cloning out of the
    /// cache is a cheap refcount bump. Invalid patterns cache as `None` so we
    /// neither recompile nor re-warn on every call.
    static COMPILED: RefCell<HashMap<String, Option<Regex>>> = RefCell::new(HashMap::new());
}

/// Compile `pattern` (or fetch the cached `Regex`). Returns `None` for an
/// invalid pattern, warning once on first compile.
fn compiled(pattern: &str) -> Option<Regex> {
    COMPILED.with(|c| {
        if let Some(r) = c.borrow().get(pattern) {
            return r.clone();
        }
        let compiled = match Regex::new(pattern) {
            Ok(r) => Some(r),
            Err(e) => {
                warn!("Invalid regex pattern '{}': {}", pattern, e);
                None
            }
        };
        c.borrow_mut().insert(pattern.to_string(), compiled.clone());
        compiled
    })
}

/// Process a regex pattern against content, returning all matches.
///
/// - If the regex contains capture groups, captured values are returned.
/// - If no capture groups, full match strings are returned.
/// - Invalid patterns return an empty vector (never panics).
pub fn process(pattern: &str, content: &str, _content_type: &ContentType) -> Vec<String> {
    let regex = match compiled(pattern) {
        Some(r) => r,
        None => return vec![],
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Guard: the same pattern compiles once and is reused, and the cache keys
    /// per distinct pattern. If the compile cache is ever removed, this fails
    /// (or stops compiling) — an uncached engine can't silently come back.
    #[test]
    fn patterns_are_compiled_once_and_cached() {
        COMPILED.with(|c| c.borrow_mut().clear());
        let _ = compiled(r"a(b)c");
        let _ = compiled(r"a(b)c");
        let _ = compiled(r"a(b)c");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 1, "repeated pattern must compile once");
        let _ = compiled(r"x(y)z");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 2, "distinct pattern adds one entry");
        // Invalid patterns are cached (as None) so they don't recompile/re-warn.
        let _ = compiled(r"(unclosed");
        let _ = compiled(r"(unclosed");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 3, "invalid pattern cached once");
    }
}
