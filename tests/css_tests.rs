//! Tests for the CSS selector extraction engine.

use chadselect::ChadSelect;

const HTML: &str = r#"
<div class="test">
    <span class="price">  $100  </span>
    <span class="price">$200</span>
    <div class="vin">VIN: 1HGCM82633A123456</div>
    <div class="info">Price: $300</div>
    <a class="link" href="https://example.com">Click here</a>
</div>
"#;

fn make_cs() -> ChadSelect {
    let mut cs = ChadSelect::new();
    cs.add_html(HTML.to_string());
    cs
}

// ─── Basic CSS selectors ────────────────────────────────────────────────────

#[test]
fn select_by_class() {
    let cs = make_cs();
    let results = cs.query(-1, "css:.price", None);
    assert_eq!(results.len(), 2);
}

#[test]
fn select_by_class_first() {
    let cs = make_cs();
    let result = cs.select(0, "css:.price");
    assert!(!result.is_empty());
}

#[test]
fn select_by_tag() {
    let cs = make_cs();
    let results = cs.query(-1, "css:a", None);
    assert_eq!(results, vec!["Click here"]);
}

// ─── Post-processing with >> ────────────────────────────────────────────────

#[test]
fn css_normalize_space() {
    let cs = make_cs();
    let results = cs.query(-1, "css:.price >> normalize-space()", None);
    assert_eq!(results, vec!["$100", "$200"]);
}

#[test]
fn css_replace_function() {
    let cs = make_cs();
    let results = cs.query(-1, r#"css:.price >> normalize-space() >> replace("$", "USD ")"#, None);
    assert_eq!(results, vec!["USD 100", "USD 200"]);
}

#[test]
fn css_substring_after() {
    let cs = make_cs();
    let results = cs.query(-1, r#"css:.vin >> substring-after("VIN: ")"#, None);
    assert_eq!(results, vec!["1HGCM82633A123456"]);
}

#[test]
fn css_substring_before() {
    let cs = make_cs();
    let results = cs.query(-1, r#"css:.info >> substring-before(": ")"#, None);
    assert_eq!(results, vec!["Price"]);
}

#[test]
fn css_chained_functions() {
    let cs = make_cs();
    let results = cs.query(
        -1,
        r#"css:.vin >> substring-after("VIN: ") >> substring(0, 3) >> lowercase()"#,
        None,
    );
    assert_eq!(results, vec!["1hg"]);
}

// ─── get-attr ───────────────────────────────────────────────────────────────

#[test]
fn css_get_attr() {
    let cs = make_cs();
    let results = cs.query(-1, "css:a.link >> get-attr('href')", None);
    assert_eq!(results, vec!["https://example.com"]);
}

#[test]
fn css_get_attr_missing() {
    let cs = make_cs();
    let results = cs.query(-1, "css:a.link >> get-attr('data-nope')", None);
    assert!(results.is_empty());
}

// ─── Text pseudo-selectors ──────────────────────────────────────────────────

const PSEUDO_HTML: &str = r#"
<div class="container">
    <div class="item">
        <div class="label">Exterior:</div>
        <div class="value">Blue Metallic</div>
    </div>
    <div class="item">
        <div class="label">Interior:</div>
        <div class="value">Black Leather</div>
    </div>
    <div class="item">
        <div class="label">Engine:</div>
        <div class="value">V6 Turbo</div>
    </div>
    <div class="other">
        <span>Not what we want</span>
    </div>
</div>
"#;

fn make_pseudo_cs() -> ChadSelect {
    let mut cs = ChadSelect::new();
    cs.add_html(PSEUDO_HTML.to_string());
    cs
}

#[test]
fn has_text_selects_ancestor() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.item:has-text('Exterior:') .value", None);
    assert_eq!(results, vec!["Blue Metallic"]);
}

#[test]
fn contains_text_matches_direct() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.label:contains-text('Interior:')", None);
    assert_eq!(results, vec!["Interior:"]);
}

#[test]
fn text_equals_exact_match() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.value:text-equals('V6 Turbo')", None);
    assert_eq!(results, vec!["V6 Turbo"]);
}

#[test]
fn text_starts_prefix_match() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.value:text-starts('Black')", None);
    assert_eq!(results, vec!["Black Leather"]);
}

#[test]
fn text_ends_suffix_match() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.value:text-ends('Metallic')", None);
    assert_eq!(results, vec!["Blue Metallic"]);
}

#[test]
fn pseudo_with_postprocess_function() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.item:has-text('Engine:') .value >> uppercase()", None);
    assert_eq!(results, vec!["V6 TURBO"]);
}

#[test]
fn pseudo_with_trim() {
    let cs = make_pseudo_cs();
    let results = cs.query(-1, "css:.item:has-text('Interior') .value >> trim()", None);
    assert_eq!(results, vec!["Black Leather"]);
}

// ─── Error handling ─────────────────────────────────────────────────────────

#[test]
fn invalid_css_selector_returns_empty() {
    let cs = make_cs();
    let results = cs.query(-1, "css:>>>invalid<<<", None);
    assert!(results.is_empty());
}

#[test]
fn css_no_match_returns_empty() {
    let cs = make_cs();
    let results = cs.query(-1, "css:.nonexistent", None);
    assert!(results.is_empty());
}

// ─── Index selection ────────────────────────────────────────────────────────

#[test]
fn css_index_first() {
    let cs = make_cs();
    let results = cs.query(0, "css:.price", None);
    assert_eq!(results.len(), 1);
}

#[test]
fn css_index_out_of_bounds() {
    let cs = make_cs();
    let results = cs.query(10, "css:.price", None);
    assert!(results.is_empty());
}
