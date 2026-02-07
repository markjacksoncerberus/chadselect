//! Post-processing text functions — shared by CSS and XPath engines.
//!
//! Functions are chained using the `>>` delimiter after a selector expression:
//! ```text
//! css:.price >> normalize-space() >> uppercase()
//! xpath://div/text() >> substring-after('VIN: ') >> substring(0, 3)
//! ```

use log::warn;

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
                let args: Vec<&str> = args_str.split(',').map(|s| s.trim()).collect();
                if args.len() >= 2 {
                    TextFunction::Replace {
                        find: args[0]
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string(),
                        replace: args[1]
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string(),
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
            _ => {
                warn!("Unknown text function: {}", func_name);
                continue;
            }
        };

        functions.push(function);
    }

    functions
}

/// Apply a chain of text functions to a vector of results.
///
/// Each function is applied to every element; elements that become empty after
/// a function are filtered out.
pub fn apply_text_functions(mut results: Vec<String>, functions: &[TextFunction]) -> Vec<String> {
    for function in functions {
        results = results
            .into_iter()
            .map(|text| apply_single_text_function(&text, function))
            .filter(|text| !text.is_empty())
            .collect();
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
    }
}
