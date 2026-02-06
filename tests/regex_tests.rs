//! Tests for the Regex extraction engine.

use chadselect::ChadSelect;

// ─── Basic matching ─────────────────────────────────────────────────────────

#[test]
fn capture_group_extracts_value() {
    let mut cs = ChadSelect::new();
    cs.add_text(r#"vehicleLat":"40.7128""#.to_string());

    let results = cs.query(-1, r#"regex:vehicleLat":"([0-9.]+)""#, None);
    assert_eq!(results, vec!["40.7128"]);
}

#[test]
fn no_capture_group_returns_full_match() {
    let mut cs = ChadSelect::new();
    cs.add_text("price: $100, price: $200".to_string());

    let results = cs.query(-1, r"regex:\$\d+", None);
    assert_eq!(results, vec!["$100", "$200"]);
}

#[test]
fn multiple_capture_groups() {
    let mut cs = ChadSelect::new();
    cs.add_text("2024-01-15".to_string());

    let results = cs.query(-1, r"regex:(\d{4})-(\d{2})-(\d{2})", None);
    assert_eq!(results, vec!["2024", "01", "15"]);
}

// ─── Index selection ────────────────────────────────────────────────────────

#[test]
fn index_all_returns_every_match() {
    let mut cs = ChadSelect::new();
    cs.add_text("price: $100, price: $200, price: $300".to_string());

    let results = cs.query(-1, r"regex:\$(\d+)", None);
    assert_eq!(results, vec!["100", "200", "300"]);
}

#[test]
fn index_zero_returns_first_match() {
    let mut cs = ChadSelect::new();
    cs.add_text("price: $100, price: $200, price: $300".to_string());

    let results = cs.query(0, r"regex:\$(\d+)", None);
    assert_eq!(results, vec!["100"]);
}

#[test]
fn index_one_returns_second_match() {
    let mut cs = ChadSelect::new();
    cs.add_text("price: $100, price: $200, price: $300".to_string());

    let results = cs.query(1, r"regex:\$(\d+)", None);
    assert_eq!(results, vec!["200"]);
}

#[test]
fn out_of_bounds_index_returns_empty() {
    let mut cs = ChadSelect::new();
    cs.add_text("price: $100".to_string());

    let results = cs.query(5, r"regex:\$(\d+)", None);
    assert!(results.is_empty());
}

// ─── Error handling ─────────────────────────────────────────────────────────

#[test]
fn invalid_regex_returns_empty() {
    let mut cs = ChadSelect::new();
    cs.add_text("test content".to_string());

    let results = cs.query(-1, r"regex:[", None);
    assert!(results.is_empty());
}

#[test]
fn no_match_returns_empty() {
    let mut cs = ChadSelect::new();
    cs.add_text("hello world".to_string());

    let results = cs.query(-1, r"regex:(\d+)", None);
    assert!(results.is_empty());
}

// ─── Cross-content ──────────────────────────────────────────────────────────

#[test]
fn regex_works_across_multiple_content_items() {
    let mut cs = ChadSelect::new();
    cs.add_text("price: $100".to_string());
    cs.add_text("price: $200".to_string());
    cs.add_html("<span>price: $300</span>".to_string());

    let results = cs.query(-1, r"regex:\$(\d+)", None);
    assert_eq!(results, vec!["100", "200", "300"]);
}

#[test]
fn regex_works_on_json_content() {
    let mut cs = ChadSelect::new();
    cs.add_json(r#"{"price": 42}"#.to_string());

    let results = cs.query(-1, r#"regex:"price":\s*(\d+)"#, None);
    assert_eq!(results, vec!["42"]);
}

// ─── Default prefix ─────────────────────────────────────────────────────────

#[test]
fn no_prefix_defaults_to_regex() {
    let mut cs = ChadSelect::new();
    cs.add_text("hello world".to_string());

    let results = cs.query(-1, r"(world)", None);
    assert_eq!(results, vec!["world"]);
}

// ─── select() convenience method ────────────────────────────────────────────

#[test]
fn select_returns_single_string() {
    let mut cs = ChadSelect::new();
    cs.add_text("lat: 40.7128".to_string());

    let result = cs.select(0, r"regex:lat:\s*([0-9.]+)");
    assert_eq!(result, "40.7128");
}

#[test]
fn select_returns_empty_string_on_no_match() {
    let mut cs = ChadSelect::new();
    cs.add_text("nothing here".to_string());

    let result = cs.select(0, r"regex:(\d+)");
    assert_eq!(result, "");
}
