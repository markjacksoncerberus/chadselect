//! Post-processing text functions — shared by CSS and XPath engines.
//!
//! Functions are chained using the `>>` delimiter after a selector expression:
//! ```text
//! css:.price >> normalize-space() >> uppercase()
//! xpath://div/text() >> substring-after('VIN: ') >> substring(0, 3)
//! ```

use log::warn;
use regex::Regex;

use crate::query::FUNCTION_PIPE;

/// Post-processing text function variants.
#[derive(Debug, Clone)]
pub enum TextFunction {
    /// Trim and collapse internal whitespace (XPath-style `normalize-space`).
    NormalizeSpace,
    /// Trim leading and trailing whitespace.
    Trim,
    /// Convert to uppercase.
    Uppercase,
    /// Convert to lowercase.
    Lowercase,
    /// Extract a substring by start index (0-based) and length.
    Substring { start: usize, length: usize },
    /// Return everything after the first occurrence of the delimiter.
    SubstringAfter { delimiter: String },
    /// Return everything before the first occurrence of the delimiter.
    SubstringBefore { delimiter: String },
    /// Replace all occurrences of `find` with `replace`.
    Replace { find: String, replace: String },
    /// Extract an HTML element attribute by name (CSS only).
    GetAttribute { attribute: String },
    /// Join **all** results in the chain into a single string with `separator`.
    /// Unlike the other functions (which map element-wise), this folds the
    /// whole result list into one value. Spelled `join('sep')` or `concat('sep')`.
    Join { separator: String },
    /// XPath-style `translate(from, to)`: per-character map; a character in
    /// `from` with no counterpart in `to` is deleted.
    Translate { from: String, to: String },
    /// Extract the first regex capture group (or the whole match if the pattern
    /// has no groups); empty string if it doesn't match.
    RegexExtract { re: Regex },
    /// Regex search-and-replace (replacement uses Rust regex `$1` group refs).
    RegexReplace { re: Regex, replace: String },
    /// Return everything after the **last** occurrence of the delimiter.
    SubstringAfterLast { delimiter: String },
    /// Return everything before the **last** occurrence of the delimiter.
    SubstringBeforeLast { delimiter: String },
}

/// Returns the list of all supported text function signatures.
pub fn supported_text_functions() -> Vec<&'static str> {
    vec![
        "normalize-space()",
        "trim()",
        "uppercase()",
        "lowercase()",
        "substring(start, length)",
        "substring-after('delimiter')",
        "substring-before('delimiter')",
        "replace('find', 'replace')",
        "get-attr('attribute')",
        "join('separator')",
        "translate('from', 'to')",
        "regex-extract('pattern')",
        "regex-replace('pattern', 'replacement')",
        "substring-after-last('delimiter')",
        "substring-before-last('delimiter')",
    ]
}

/// Split an input string on the function-pipe delimiter (`>>`).
///
/// Returns `(expression, functions)` where `expression` is the selector/query
/// portion and `functions` is the parsed chain of [`TextFunction`]s.
///
/// If no `>>` is present, the entire input is treated as the expression with
/// an empty function chain.
pub fn split_functions(input: &str) -> (&str, Vec<TextFunction>) {
    if let Some(pipe_pos) = input.find(FUNCTION_PIPE) {
        let expression = input[..pipe_pos].trim();
        let functions_str = &input[pipe_pos + FUNCTION_PIPE.len()..];
        let functions = parse_text_functions(functions_str);
        (expression, functions)
    } else {
        (input, vec![])
    }
}

/// Parse a function chain string like `"normalize-space() >> uppercase()"`.
///
/// Individual function strings that are malformed or unrecognised are silently
/// skipped (with a `log::warn`), keeping the rest of the chain intact.
pub fn parse_text_functions(functions_str: &str) -> Vec<TextFunction> {
    let mut functions = Vec::new();

    for func_str in functions_str.split(FUNCTION_PIPE) {
        let func_str = func_str.trim();
        if func_str.is_empty() {
            continue;
        }

        let Some(paren_pos) = func_str.find('(') else {
            continue;
        };

        let func_name = func_str[..paren_pos].trim();
        let args_str = &func_str[paren_pos + 1..];
        let args_end = args_str.rfind(')').unwrap_or(args_str.len());
        let args_str = &args_str[..args_end];

        let function = match func_name {
            "normalize-space" => TextFunction::NormalizeSpace,
            "trim" => TextFunction::Trim,
            "uppercase" => TextFunction::Uppercase,
            "lowercase" => TextFunction::Lowercase,
            "substring" => {
                let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
                if args.len() >= 2 {
                    if let (Ok(start), Ok(length)) =
                        (args[0].parse::<usize>(), args[1].parse::<usize>())
                    {
                        TextFunction::Substring { start, length }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            "substring-after" => {
                if !args_str.is_empty() {
                    TextFunction::SubstringAfter {
                        delimiter: args_str
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string(),
                    }
                } else {
                    continue;
                }
            }
            "substring-before" => {
                if !args_str.is_empty() {
                    TextFunction::SubstringBefore {
                        delimiter: args_str
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string(),
                    }
                } else {
                    continue;
                }
            }
            "replace" => {
                // Quote-aware so `replace(',', '')` (comma in an arg) works.
                if let Some((find, replace)) = parse_two_quoted(args_str) {
                    TextFunction::Replace { find, replace }
                } else {
                    continue;
                }
            }
            "translate" => {
                if let Some((from, to)) = parse_two_quoted(args_str) {
                    TextFunction::Translate { from, to }
                } else {
                    continue;
                }
            }
            "regex-extract" => {
                let pat = args_str.trim().trim_matches('"').trim_matches('\'');
                match Regex::new(pat) {
                    Ok(re) => TextFunction::RegexExtract { re },
                    Err(e) => {
                        warn!("Invalid regex in regex-extract('{}'): {}", pat, e);
                        continue;
                    }
                }
            }
            "regex-replace" => {
                if let Some((pat, replace)) = parse_two_quoted(args_str) {
                    match Regex::new(&pat) {
                        Ok(re) => TextFunction::RegexReplace { re, replace },
                        Err(e) => {
                            warn!("Invalid regex in regex-replace('{}'): {}", pat, e);
                            continue;
                        }
                    }
                } else {
                    continue;
                }
            }
            "substring-after-last" => {
                if !args_str.is_empty() {
                    TextFunction::SubstringAfterLast {
                        delimiter: args_str.trim_matches('"').trim_matches('\'').to_string(),
                    }
                } else {
                    continue;
                }
            }
            "substring-before-last" => {
                if !args_str.is_empty() {
                    TextFunction::SubstringBeforeLast {
                        delimiter: args_str.trim_matches('"').trim_matches('\'').to_string(),
                    }
                } else {
                    continue;
                }
            }
            "get-attr" => {
                if !args_str.is_empty() {
                    TextFunction::GetAttribute {
                        attribute: args_str
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string(),
                    }
                } else {
                    continue;
                }
            }
            // `join('sep')` / `concat('sep')` — fold the result list into one
            // string. An empty/absent argument joins with no separator.
            "join" | "concat" => TextFunction::Join {
                separator: args_str.trim_matches('"').trim_matches('\'').to_string(),
            },
            _ => {
                warn!("Unknown text function: {}", func_name);
                continue;
            }
        };

        functions.push(function);
    }

    functions
}

/// Extract two quoted string arguments (single or double quotes) from an
/// argument list, e.g. `'find', 'replace'`. Quote-aware, so a comma *inside*
/// an argument (`',', ''`) is not mistaken for the argument separator.
fn parse_two_quoted(args_str: &str) -> Option<(String, String)> {
    let chars: Vec<char> = args_str.chars().collect();
    let mut i = 0;
    let first = read_quoted(&chars, &mut i)?;
    let second = read_quoted(&chars, &mut i)?;
    Some((first, second))
}

/// Read the next single/double-quoted string starting at or after `*i`,
/// advancing `*i` past the closing quote.
fn read_quoted(chars: &[char], i: &mut usize) -> Option<String> {
    while *i < chars.len() && chars[*i] != '\'' && chars[*i] != '"' {
        *i += 1;
    }
    let quote = *chars.get(*i)?;
    *i += 1;
    let start = *i;
    while *i < chars.len() && chars[*i] != quote {
        *i += 1;
    }
    if *i >= chars.len() {
        return None; // unterminated
    }
    let value: String = chars[start..*i].iter().collect();
    *i += 1; // consume closing quote
    Some(value)
}

/// Apply a chain of text functions to a vector of results.
///
/// Each function is applied to every element; elements that become empty after
/// a function are filtered out.
pub fn apply_text_functions(mut results: Vec<String>, functions: &[TextFunction]) -> Vec<String> {
    for function in functions {
        match function {
            // Fold: join the whole list into a single result.
            TextFunction::Join { separator } => {
                let joined = results.join(separator.as_str());
                results = if joined.is_empty() { vec![] } else { vec![joined] };
            }
            // Map: transform each element, dropping any that become empty.
            _ => {
                results = results
                    .into_iter()
                    .map(|text| apply_single_text_function(&text, function))
                    .filter(|text| !text.is_empty())
                    .collect();
            }
        }
    }
    results
}

/// Apply a single text function to a string.
pub fn apply_single_text_function(text: &str, function: &TextFunction) -> String {
    match function {
        TextFunction::NormalizeSpace => {
            text.split_whitespace().collect::<Vec<_>>().join(" ")
        }
        TextFunction::Trim => text.trim().to_string(),
        TextFunction::Uppercase => text.to_uppercase(),
        TextFunction::Lowercase => text.to_lowercase(),
        TextFunction::Substring { start, length } => {
            let chars: Vec<char> = text.chars().collect();
            if *start < chars.len() {
                let end = (*start + *length).min(chars.len());
                chars[*start..end].iter().collect()
            } else {
                String::new()
            }
        }
        TextFunction::SubstringAfter { delimiter } => {
            if let Some(pos) = text.find(delimiter.as_str()) {
                text[pos + delimiter.len()..].to_string()
            } else {
                String::new()
            }
        }
        TextFunction::SubstringBefore { delimiter } => {
            if let Some(pos) = text.find(delimiter.as_str()) {
                text[..pos].to_string()
            } else {
                text.to_string()
            }
        }
        TextFunction::Replace { find, replace } => text.replace(find.as_str(), replace.as_str()),
        TextFunction::GetAttribute { .. } => {
            // Handled specially during CSS processing, not as a generic text function.
            text.to_string()
        }
        TextFunction::Join { .. } => {
            // Folds the whole list; handled in `apply_text_functions`, not here.
            text.to_string()
        }
        TextFunction::Translate { from, to } => {
            let to_chars: Vec<char> = to.chars().collect();
            text.chars()
                .filter_map(|c| match from.chars().position(|fc| fc == c) {
                    // Mapped char, or deleted when `to` has no counterpart.
                    Some(idx) => to_chars.get(idx).copied(),
                    None => Some(c),
                })
                .collect()
        }
        TextFunction::RegexExtract { re } => match re.captures(text) {
            Some(caps) => caps
                .get(1)
                .or_else(|| caps.get(0))
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
            None => String::new(),
        },
        TextFunction::RegexReplace { re, replace } => {
            re.replace_all(text, replace.as_str()).into_owned()
        }
        TextFunction::SubstringAfterLast { delimiter } => match text.rfind(delimiter.as_str()) {
            Some(pos) => text[pos + delimiter.len()..].to_string(),
            None => String::new(),
        },
        TextFunction::SubstringBeforeLast { delimiter } => match text.rfind(delimiter.as_str()) {
            Some(pos) => text[..pos].to_string(),
            None => text.to_string(),
        },
    }
}
