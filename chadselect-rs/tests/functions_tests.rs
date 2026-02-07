//! Tests for post-processing text functions.

use chadselect::functions::{
    apply_single_text_function, apply_text_functions, parse_text_functions, TextFunction,
};

// ─── Parsing ────────────────────────────────────────────────────────────────

#[test]
fn parse_normalize_space() {
    let functions = parse_text_functions("normalize-space()");
    assert_eq!(functions.len(), 1);
    assert!(matches!(functions[0], TextFunction::NormalizeSpace));
}

#[test]
fn parse_trim() {
    let functions = parse_text_functions("trim()");
    assert_eq!(functions.len(), 1);
    assert!(matches!(functions[0], TextFunction::Trim));
}

#[test]
fn parse_case_functions() {
    let functions = parse_text_functions("uppercase() >> lowercase()");
    assert_eq!(functions.len(), 2);
    assert!(matches!(functions[0], TextFunction::Uppercase));
    assert!(matches!(functions[1], TextFunction::Lowercase));
}

#[test]
fn parse_substring() {
    let functions = parse_text_functions("substring(1, 3)");
    assert_eq!(functions.len(), 1);
    if let TextFunction::Substring { start, length } = &functions[0] {
        assert_eq!(*start, 1);
        assert_eq!(*length, 3);
    } else {
        panic!("Expected Substring function");
    }
}

#[test]
fn parse_substring_after() {
    let functions = parse_text_functions("substring-after('VIN: ')");
    assert_eq!(functions.len(), 1);
    if let TextFunction::SubstringAfter { delimiter } = &functions[0] {
        assert_eq!(delimiter, "VIN: ");
    } else {
        panic!("Expected SubstringAfter function");
    }
}

#[test]
fn parse_substring_before() {
    let functions = parse_text_functions("substring-before(': ')");
    assert_eq!(functions.len(), 1);
    if let TextFunction::SubstringBefore { delimiter } = &functions[0] {
        assert_eq!(delimiter, ": ");
    } else {
        panic!("Expected SubstringBefore function");
    }
}

#[test]
fn parse_replace() {
    let functions = parse_text_functions(r#"replace("$", "USD ")"#);
    assert_eq!(functions.len(), 1);
    if let TextFunction::Replace { find, replace } = &functions[0] {
        assert_eq!(find, "$");
        assert_eq!(replace, "USD ");
    } else {
        panic!("Expected Replace function");
    }
}

#[test]
fn parse_unknown_function_is_skipped() {
    let functions = parse_text_functions("invalid-function()");
    assert_eq!(functions.len(), 0);
}

#[test]
fn parse_incomplete_args_are_skipped() {
    let functions = parse_text_functions(r#"substring(1) >> replace("only-one-arg")"#);
    assert_eq!(functions.len(), 0);
}

// ─── Apply single function ──────────────────────────────────────────────────

#[test]
fn apply_normalize_space() {
    assert_eq!(
        apply_single_text_function("  Hello   World  ", &TextFunction::NormalizeSpace),
        "Hello World"
    );
    assert_eq!(
        apply_single_text_function("\n\tTest\r\n  Text\t", &TextFunction::NormalizeSpace),
        "Test Text"
    );
}

#[test]
fn apply_trim() {
    assert_eq!(
        apply_single_text_function("  Hello World  ", &TextFunction::Trim),
        "Hello World"
    );
}

#[test]
fn apply_uppercase() {
    assert_eq!(
        apply_single_text_function("Hello World", &TextFunction::Uppercase),
        "HELLO WORLD"
    );
}

#[test]
fn apply_lowercase() {
    assert_eq!(
        apply_single_text_function("Hello World", &TextFunction::Lowercase),
        "hello world"
    );
}

#[test]
fn apply_substring() {
    assert_eq!(
        apply_single_text_function(
            "Hello World",
            &TextFunction::Substring {
                start: 0,
                length: 5,
            }
        ),
        "Hello"
    );
    assert_eq!(
        apply_single_text_function(
            "Hello World",
            &TextFunction::Substring {
                start: 6,
                length: 5,
            }
        ),
        "World"
    );
}

#[test]
fn apply_substring_out_of_bounds() {
    assert_eq!(
        apply_single_text_function(
            "Hello",
            &TextFunction::Substring {
                start: 10,
                length: 5,
            }
        ),
        ""
    );
    assert_eq!(
        apply_single_text_function(
            "Hello",
            &TextFunction::Substring {
                start: 3,
                length: 10,
            }
        ),
        "lo"
    );
}

#[test]
fn apply_substring_after() {
    assert_eq!(
        apply_single_text_function(
            "VIN: 1HGCM82633A123456",
            &TextFunction::SubstringAfter {
                delimiter: "VIN: ".to_string(),
            }
        ),
        "1HGCM82633A123456"
    );
    assert_eq!(
        apply_single_text_function(
            "Price: $25,000",
            &TextFunction::SubstringAfter {
                delimiter: ": $".to_string(),
            }
        ),
        "25,000"
    );
}

#[test]
fn apply_substring_after_missing_delimiter() {
    assert_eq!(
        apply_single_text_function(
            "Hello World",
            &TextFunction::SubstringAfter {
                delimiter: "XYZ".to_string(),
            }
        ),
        ""
    );
}

#[test]
fn apply_substring_before() {
    assert_eq!(
        apply_single_text_function(
            "Price: $25,000",
            &TextFunction::SubstringBefore {
                delimiter: ": ".to_string(),
            }
        ),
        "Price"
    );
    assert_eq!(
        apply_single_text_function(
            "user@domain.com",
            &TextFunction::SubstringBefore {
                delimiter: "@".to_string(),
            }
        ),
        "user"
    );
}

#[test]
fn apply_substring_before_missing_delimiter() {
    assert_eq!(
        apply_single_text_function(
            "Hello World",
            &TextFunction::SubstringBefore {
                delimiter: "XYZ".to_string(),
            }
        ),
        "Hello World"
    );
}

#[test]
fn apply_replace() {
    assert_eq!(
        apply_single_text_function(
            "$100",
            &TextFunction::Replace {
                find: "$".to_string(),
                replace: "USD ".to_string(),
            }
        ),
        "USD 100"
    );
    assert_eq!(
        apply_single_text_function(
            "Hello Hello World",
            &TextFunction::Replace {
                find: "Hello".to_string(),
                replace: "Hi".to_string(),
            }
        ),
        "Hi Hi World"
    );
}

#[test]
fn apply_replace_no_match() {
    assert_eq!(
        apply_single_text_function(
            "Hello World",
            &TextFunction::Replace {
                find: "XYZ".to_string(),
                replace: "ABC".to_string(),
            }
        ),
        "Hello World"
    );
}

// ─── Function chaining ─────────────────────────────────────────────────────

#[test]
fn chain_substring_after_then_substring_then_lowercase() {
    let functions = vec![
        TextFunction::SubstringAfter {
            delimiter: "VIN: ".to_string(),
        },
        TextFunction::Substring {
            start: 0,
            length: 3,
        },
        TextFunction::Lowercase,
    ];
    let results =
        apply_text_functions(vec!["VIN: 1HGCM82633A123456".to_string()], &functions);
    assert_eq!(results, vec!["1hg"]);
}

#[test]
fn chain_normalize_trim_uppercase() {
    let functions = vec![
        TextFunction::NormalizeSpace,
        TextFunction::Trim,
        TextFunction::Uppercase,
    ];
    let results = apply_text_functions(vec!["  Hello   World  ".to_string()], &functions);
    assert_eq!(results, vec!["HELLO WORLD"]);
}

#[test]
fn chain_empty_result_filters_out() {
    let functions = vec![TextFunction::SubstringAfter {
        delimiter: "MISSING".to_string(),
    }];
    let results = apply_text_functions(vec!["Hello World".to_string()], &functions);
    assert!(results.is_empty());
}

// ─── Unicode ────────────────────────────────────────────────────────────────

#[test]
fn unicode_normalize_space() {
    assert_eq!(
        apply_single_text_function("  Hello   🌍  ", &TextFunction::NormalizeSpace),
        "Hello 🌍"
    );
}

#[test]
fn unicode_substring() {
    assert_eq!(
        apply_single_text_function(
            "Hello 🌍 World",
            &TextFunction::Substring {
                start: 6,
                length: 1,
            }
        ),
        "🌍"
    );
}

#[test]
fn unicode_substring_after() {
    assert_eq!(
        apply_single_text_function(
            "価格: ¥1000",
            &TextFunction::SubstringAfter {
                delimiter: "価格: ".to_string(),
            }
        ),
        "¥1000"
    );
}

#[test]
fn unicode_uppercase() {
    assert_eq!(
        apply_single_text_function("Héllo Wörld", &TextFunction::Uppercase),
        "HÉLLO WÖRLD"
    );
}

// ─── Edge cases ─────────────────────────────────────────────────────────────

#[test]
fn empty_string_normalize_space() {
    assert_eq!(
        apply_single_text_function("", &TextFunction::NormalizeSpace),
        ""
    );
}

#[test]
fn empty_string_substring_after() {
    assert_eq!(
        apply_single_text_function(
            "",
            &TextFunction::SubstringAfter {
                delimiter: "test".to_string(),
            }
        ),
        ""
    );
}

#[test]
fn empty_string_substring_before() {
    assert_eq!(
        apply_single_text_function(
            "",
            &TextFunction::SubstringBefore {
                delimiter: "test".to_string(),
            }
        ),
        ""
    );
}

#[test]
fn very_long_string() {
    let long_string = "A".repeat(10_000);
    let result = apply_single_text_function(&long_string, &TextFunction::Trim);
    assert_eq!(result.len(), 10_000);
}

#[test]
fn delimiter_longer_than_input() {
    assert_eq!(
        apply_single_text_function(
            "Hi",
            &TextFunction::SubstringAfter {
                delimiter: "This is much longer".to_string(),
            }
        ),
        ""
    );
}

#[test]
fn delimiter_at_boundaries() {
    assert_eq!(
        apply_single_text_function(
            "VIN: 123",
            &TextFunction::SubstringAfter {
                delimiter: "VIN: ".to_string(),
            }
        ),
        "123"
    );
    assert_eq!(
        apply_single_text_function(
            "123: END",
            &TextFunction::SubstringBefore {
                delimiter: ": END".to_string(),
            }
        ),
        "123"
    );
}
