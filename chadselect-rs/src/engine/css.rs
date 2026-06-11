//! CSS selector extraction engine.
//!
//! Processes CSS selectors against HTML content using the `scraper` crate.
//! Supports standard CSS selectors, custom text pseudo-selectors for
//! content-based filtering, and post-processing text functions.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use log::warn;
use scraper::{ElementRef, Html, Selector};

use crate::content::ContentItem;
use crate::functions::{self, TextFunction};

thread_local! {
    /// Cache of parsed CSS selectors, keyed by the selector string.
    ///
    /// `Selector::parse` runs the CSS parser on every call; a crawler reuses the
    /// same selectors across many documents, so parsing once per distinct
    /// selector (per thread) removes that repeated cost. `Selector` is `Clone`,
    /// so we clone out of the cache. Invalid selectors cache as `None` to avoid
    /// re-parsing / re-warning each call.
    static COMPILED: RefCell<HashMap<String, Option<Selector>>> = RefCell::new(HashMap::new());
}

/// Parse `selector` (or fetch the cached `Selector`). Returns `None` for an
/// invalid selector; the caller logs context-specific warnings.
fn cached_selector(selector: &str) -> Option<Selector> {
    COMPILED.with(|c| {
        if let Some(s) = c.borrow().get(selector) {
            return s.clone();
        }
        let parsed = Selector::parse(selector).ok();
        c.borrow_mut().insert(selector.to_string(), parsed.clone());
        parsed
    })
}

// ─── Text pseudo-selector types ─────────────────────────────────────────────

/// Text pseudo-selector for filtering elements by their text content.
#[derive(Debug, Clone)]
enum TextPseudoSelector {
    /// Element or its descendants contain the text.
    HasText(String),
    /// Element's direct text content contains the text.
    ContainsText(String),
    /// Element's text content exactly equals the text.
    TextEquals(String),
    /// Element's text content starts with the text.
    TextStarts(String),
    /// Element's text content ends with the text.
    TextEnds(String),
}

/// Parsed CSS selector with optional text pseudo-selector and function chain.
#[derive(Debug, Clone)]
struct ParsedCssSelector {
    /// The main CSS selector before any text pseudo-selector.
    base_selector: String,
    /// Optional text-based filter.
    text_pseudo: Option<TextPseudoSelector>,
    /// CSS selector to apply to descendants of filtered elements.
    post_selector: String,
    /// Post-processing function chain.
    functions: Vec<TextFunction>,
}

/// All recognised text pseudo-selector prefixes.
const PSEUDO_PATTERNS: [&str; 5] = [
    ":has-text(",
    ":contains-text(",
    ":text-equals(",
    ":text-starts(",
    ":text-ends(",
];

// ─── Public entry point ─────────────────────────────────────────────────────

/// Process a CSS selector expression (potentially with `>>` function chain
/// and text pseudo-selectors) against a content item, returning matches.
pub fn process(selector_with_functions: &str, content_item: &ContentItem) -> Vec<String> {
    // Route to text-pseudo path if any pseudo-selector is present.
    if PSEUDO_PATTERNS
        .iter()
        .any(|p| selector_with_functions.contains(p))
    {
        return process_with_text_selectors(selector_with_functions, content_item);
    }

    process_standard(selector_with_functions, content_item)
}

// ─── Standard CSS selector processing ───────────────────────────────────────

/// Standard CSS selector processing (no text pseudo-selectors).
fn process_standard(selector_with_functions: &str, content_item: &ContentItem) -> Vec<String> {
    let (css_selector_str, text_functions) = functions::split_functions(selector_with_functions);

    // Use the shared, already-parsed HTML document.
    let html_doc = content_item.html();

    let css_selector = match cached_selector(css_selector_str) {
        Some(s) => s,
        None => {
            warn!("Invalid CSS selector '{}'", css_selector_str);
            return vec![];
        }
    };

    let selected_elements: Vec<_> = html_doc.select(&css_selector).collect();

    let has_get_attr = text_functions
        .iter()
        .any(|f| matches!(f, TextFunction::GetAttribute { .. }));

    let mut results: Vec<String> = if has_get_attr {
        selected_elements
            .iter()
            .flat_map(|element| {
                for function in &text_functions {
                    if let TextFunction::GetAttribute { attribute } = function {
                        return vec![element
                            .value()
                            .attr(attribute)
                            .unwrap_or("")
                            .to_string()];
                    }
                }
                vec![String::new()]
            })
            .filter(|text| !text.is_empty())
            .collect()
    } else {
        selected_elements
            .iter()
            .map(|element| {
                element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string()
            })
            .filter(|text| !text.is_empty())
            .collect()
    };

    // Apply non-get-attr functions.
    if !text_functions.is_empty() {
        let fns_to_apply: Vec<_> = if has_get_attr {
            text_functions
                .iter()
                .filter(|f| !matches!(f, TextFunction::GetAttribute { .. }))
                .cloned()
                .collect()
        } else {
            text_functions
        };
        if !fns_to_apply.is_empty() {
            results = functions::apply_text_functions(results, &fns_to_apply);
        }
    }

    results
}

// ─── Text pseudo-selector processing ────────────────────────────────────────

/// Process CSS selectors with text pseudo-selectors (two-stage approach).
fn process_with_text_selectors(
    selector_with_functions: &str,
    content_item: &ContentItem,
) -> Vec<String> {
    let parsed = parse_with_text_selectors(selector_with_functions);

    // Get cached element data (populates cache as a side-effect).
    let cached_data = if !parsed.base_selector.is_empty() {
        get_cached_elements_data(&parsed.base_selector, content_item)
    } else {
        Vec::new()
    };

    // Use the shared, already-parsed HTML document.
    let html_doc = content_item.html();

    // Stage 1: Resolve base elements.
    let (base_elements, element_texts): (Vec<_>, Vec<_>) = if parsed.base_selector.is_empty() {
        let star = cached_selector("*").expect("'*' is a valid selector");
        let elements: Vec<_> = html_doc.select(&star).collect();
        let texts: Vec<_> = elements
            .iter()
            .map(|e| e.text().collect::<Vec<_>>().join(" ").trim().to_string())
            .collect();
        (elements, texts)
    } else if !cached_data.is_empty() {
        match cached_selector(&parsed.base_selector) {
            Some(selector) => {
                let all_elements: Vec<_> = html_doc.select(&selector).collect();
                let mut elements = Vec::new();
                let mut texts = Vec::new();
                for (index, text_content) in &cached_data {
                    if let Some(element) = all_elements.get(*index) {
                        elements.push(*element);
                        texts.push(text_content.clone());
                    }
                }
                (elements, texts)
            }
            None => {
                warn!("Invalid base CSS selector '{}'", parsed.base_selector);
                return vec![];
            }
        }
    } else {
        match cached_selector(&parsed.base_selector) {
            Some(selector) => {
                let elements: Vec<_> = html_doc.select(&selector).collect();
                let texts: Vec<_> = elements
                    .iter()
                    .map(|e| e.text().collect::<Vec<_>>().join(" ").trim().to_string())
                    .collect();
                (elements, texts)
            }
            None => {
                warn!("Invalid base CSS selector '{}'", parsed.base_selector);
                return vec![];
            }
        }
    };

    // Stage 2: Filter by text pseudo-selector.
    let filtered_elements: Vec<_> = if let Some(text_pseudo) = &parsed.text_pseudo {
        base_elements
            .into_iter()
            .zip(element_texts)
            .filter(|(_, text_content)| match text_pseudo {
                TextPseudoSelector::HasText(t) => text_content.contains(t.as_str()),
                TextPseudoSelector::ContainsText(t) => text_content.contains(t.as_str()),
                TextPseudoSelector::TextEquals(t) => text_content == t,
                TextPseudoSelector::TextStarts(t) => text_content.starts_with(t.as_str()),
                TextPseudoSelector::TextEnds(t) => text_content.ends_with(t.as_str()),
            })
            .map(|(element, _)| element)
            .collect()
    } else {
        base_elements
    };

    // Stage 3: Apply the post-selector relative to the text-matched elements.
    // Supports a descendant post (default) plus the `+`, `~`, and `>`
    // combinators — so idioms like `span:text-equals(Label) + span` work
    // (previously the post was descendant-only and a leading combinator was
    // silently dropped, returning nothing).
    let final_elements = apply_post_selector(&html_doc, filtered_elements, &parsed.post_selector);

    // Extract text from final elements.
    let mut results: Vec<String> = final_elements
        .iter()
        .map(|element| {
            element
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string()
        })
        .filter(|text| !text.is_empty())
        .collect();

    if !parsed.functions.is_empty() {
        results = functions::apply_text_functions(results, &parsed.functions);
    }

    results
}

/// Apply a post-selector to the text-matched elements, honouring a leading
/// combinator. With no leading combinator the post is treated as a descendant
/// selector (the historical behaviour); `+`/`~`/`>` navigate to the adjacent
/// sibling / following siblings / children of each matched element.
///
/// Combinator cases select the post compound globally and keep candidates whose
/// relationship target is one of the matched elements — so the post selector
/// (`rest`) can be any compound scraper understands.
fn apply_post_selector<'a>(
    doc: &'a Html,
    matched: Vec<ElementRef<'a>>,
    post: &str,
) -> Vec<ElementRef<'a>> {
    let post = post.trim();
    if post.is_empty() {
        return matched;
    }

    let (combinator, rest) = match post.chars().next() {
        Some('+') => ('+', post[1..].trim()),
        Some('~') => ('~', post[1..].trim()),
        Some('>') => ('>', post[1..].trim()),
        _ => (' ', post), // descendant
    };

    let selector = match cached_selector(rest) {
        Some(s) => s,
        None => {
            warn!("Invalid post CSS selector '{}'", rest);
            return Vec::new();
        }
    };

    // Descendant post: keep the per-matched-element selection (unchanged).
    if combinator == ' ' {
        return matched
            .iter()
            .flat_map(|el| el.select(&selector))
            .collect();
    }

    let matched_ids: HashSet<_> = matched.iter().map(|e| e.id()).collect();
    doc.select(&selector)
        .filter(|cand| match combinator {
            // Immediate previous element sibling is a matched element.
            '+' => cand
                .prev_siblings()
                .find_map(ElementRef::wrap)
                .is_some_and(|p| matched_ids.contains(&p.id())),
            // Some previous element sibling is a matched element.
            '~' => cand
                .prev_siblings()
                .filter_map(ElementRef::wrap)
                .any(|p| matched_ids.contains(&p.id())),
            // Parent element is a matched element.
            '>' => cand
                .parent()
                .and_then(ElementRef::wrap)
                .is_some_and(|p| matched_ids.contains(&p.id())),
            _ => false,
        })
        .collect()
}

// ─── Element-text caching ───────────────────────────────────────────────────

/// Retrieve or populate the element-text cache for a base selector.
fn get_cached_elements_data(
    base_selector: &str,
    content_item: &ContentItem,
) -> Vec<(usize, String)> {
    // Check cache first.
    {
        let cache = content_item.element_text_cache.borrow();
        if let Some(cached_data) = cache.get(base_selector) {
            return cached_data.clone();
        }
    }

    // Cache miss — query the shared document and store.
    let html_doc = content_item.html();
    if let Some(selector) = cached_selector(base_selector) {
        let elements: Vec<_> = html_doc.select(&selector).collect();
        let cache_data: Vec<_> = elements
            .iter()
            .enumerate()
            .map(|(index, element)| {
                let text = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                (index, text)
            })
            .collect();

        let mut cache = content_item.element_text_cache.borrow_mut();
        cache.insert(base_selector.to_string(), cache_data.clone());
        return cache_data;
    }

    Vec::new()
}

// ─── Selector parsing ───────────────────────────────────────────────────────

/// Parse a CSS selector string that may contain text pseudo-selectors and
/// a `>>` function chain into a structured representation.
fn parse_with_text_selectors(input: &str) -> ParsedCssSelector {
    let (base_input, text_functions) = functions::split_functions(input);

    let mut text_pseudo = None;
    let mut base_selector = String::new();
    let mut post_selector = String::new();
    let mut found_pseudo = false;

    for pattern in PSEUDO_PATTERNS.iter() {
        if let Some(start_pos) = base_input.find(pattern) {
            let content_start = start_pos + pattern.len();
            let mut paren_count = 1;
            let mut end_pos = content_start;

            for (i, ch) in base_input[content_start..].char_indices() {
                match ch {
                    '(' => paren_count += 1,
                    ')' => {
                        paren_count -= 1;
                        if paren_count == 0 {
                            end_pos = content_start + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if paren_count == 0 {
                let text_content = base_input[content_start..end_pos].trim();
                let clean_text = text_content
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();

                text_pseudo = Some(match *pattern {
                    ":has-text(" => TextPseudoSelector::HasText(clean_text),
                    ":contains-text(" => TextPseudoSelector::ContainsText(clean_text),
                    ":text-equals(" => TextPseudoSelector::TextEquals(clean_text),
                    ":text-starts(" => TextPseudoSelector::TextStarts(clean_text),
                    ":text-ends(" => TextPseudoSelector::TextEnds(clean_text),
                    _ => continue,
                });

                base_selector = base_input[..start_pos].trim().to_string();
                post_selector = base_input[end_pos + 1..].to_string();
                found_pseudo = true;
                break;
            }
        }
    }

    if !found_pseudo {
        base_selector = base_input.to_string();
    }

    ParsedCssSelector {
        base_selector,
        text_pseudo,
        post_selector,
        functions: text_functions,
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    /// Guard: CSS selectors parse once and are cached per distinct selector.
    /// Fails (or stops compiling) if the parse cache is removed.
    #[test]
    fn selectors_are_parsed_once_and_cached() {
        COMPILED.with(|c| c.borrow_mut().clear());
        let _ = cached_selector("div.product");
        let _ = cached_selector("div.product");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 1, "repeated selector must parse once");
        let _ = cached_selector("span.price");
        assert_eq!(COMPILED.with(|c| c.borrow().len()), 2, "distinct selector adds one entry");
    }
}
