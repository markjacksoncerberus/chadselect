//! Tests for the JMESPath extraction engine.

use chadselect::ChadSelect;

const JSON: &str = r#"{
    "store": {
        "name": "Widget World",
        "inventory": [
            {"name": "Widget", "price": 25, "in_stock": true},
            {"name": "Gadget", "price": 50, "in_stock": false},
            {"name": "Doohickey", "price": 10, "in_stock": true}
        ]
    }
}"#;

fn make_cs() -> ChadSelect {
    let mut cs = ChadSelect::new();
    cs.add_json(JSON.to_string());
    cs
}

// ─── Simple paths ───────────────────────────────────────────────────────────

#[test]
fn simple_string_path() {
    let cs = make_cs();
    let result = cs.select(0, "json:store.name");
    assert_eq!(result, "Widget World");
}

#[test]
fn nested_array_index() {
    let cs = make_cs();
    let result = cs.select(0, "json:store.inventory[0].name");
    assert_eq!(result, "Widget");
}

#[test]
fn number_value() {
    let cs = make_cs();
    let result = cs.select(0, "json:store.inventory[0].price");
    assert_eq!(result, "25");
}

#[test]
fn boolean_value() {
    let cs = make_cs();
    let result = cs.select(0, "json:store.inventory[1].in_stock");
    assert_eq!(result, "false");
}

// ─── Projections ────────────────────────────────────────────────────────────

#[test]
fn array_projection_names() {
    let cs = make_cs();
    let results = cs.query(-1, "json:store.inventory[].name");
    assert_eq!(results, vec!["Widget", "Gadget", "Doohickey"]);
}

#[test]
fn array_projection_prices() {
    let cs = make_cs();
    let results = cs.query(-1, "json:store.inventory[].price");
    assert_eq!(results, vec!["25", "50", "10"]);
}

// ─── Error handling ─────────────────────────────────────────────────────────

#[test]
fn invalid_jmespath_returns_empty() {
    let cs = make_cs();
    let results = cs.query(-1, "json:`invalid");
    assert!(results.is_empty());
}

#[test]
fn jmespath_no_match_returns_empty() {
    let cs = make_cs();
    let result = cs.select(0, "json:nonexistent.path");
    assert_eq!(result, "");
}

#[test]
fn invalid_json_content_returns_empty() {
    let mut cs = ChadSelect::new();
    cs.add_json("not json at all".to_string());

    let results = cs.query(-1, "json:whatever");
    assert!(results.is_empty());
}

// ─── Index selection ────────────────────────────────────────────────────────

#[test]
fn json_index_first() {
    let cs = make_cs();
    let results = cs.query(0, "json:store.inventory[].name");
    assert_eq!(results, vec!["Widget"]);
}

#[test]
fn json_index_last() {
    let cs = make_cs();
    let results = cs.query(2, "json:store.inventory[].name");
    assert_eq!(results, vec!["Doohickey"]);
}

#[test]
fn json_index_out_of_bounds() {
    let cs = make_cs();
    let results = cs.query(10, "json:store.inventory[].name");
    assert!(results.is_empty());
}

// ─── Content type routing ───────────────────────────────────────────────────

#[test]
fn json_query_skips_html_content() {
    let mut cs = ChadSelect::new();
    cs.add_html("<div>hello</div>".to_string());

    let results = cs.query(-1, "json:whatever");
    assert!(results.is_empty());
}

#[test]
fn json_query_skips_text_content() {
    let mut cs = ChadSelect::new();
    cs.add_text("hello".to_string());

    let results = cs.query(-1, "json:whatever");
    assert!(results.is_empty());
}
