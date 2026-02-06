//! Integration tests — cross-engine and multi-content scenarios.

use chadselect::{ChadSelect, ContentType};

// ─── select_first fallback chains ───────────────────────────────────────────

#[test]
fn select_first_returns_first_hit() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<span id="vin">ABC123</span>"#.to_string());

    let result = cs.select_first(vec![
        (0, "css:#nonexistent"),               // miss
        (0, "xpath://span[@id='vin']/text()"), // hit
        (0, "css:#vin"),                        // would also hit, but skipped
    ]);
    assert_eq!(result, vec!["ABC123"]);
}

#[test]
fn select_first_returns_empty_when_all_miss() {
    let mut cs = ChadSelect::new();
    cs.add_text("nothing useful".to_string());

    let result = cs.select_first(vec![
        (0, "css:.nope"),
        (0, r"regex:(\d+)"),
    ]);
    assert!(result.is_empty());
}

// ─── select_many combines results ───────────────────────────────────────────

#[test]
fn select_many_combines_unique_results() {
    let mut cs = ChadSelect::new();
    cs.add_html(
        r#"
        <span class="a">Alpha</span>
        <span class="b">Beta</span>
        <span class="c">Alpha</span>
    "#
        .to_string(),
    );

    let results = cs.select_many(vec![(-1, "css:.a"), (-1, "css:.b"), (-1, "css:.c")]);
    // "Alpha" should appear only once (deduped)
    assert!(results.contains(&"Alpha".to_string()));
    assert!(results.contains(&"Beta".to_string()));
    assert_eq!(results.len(), 2);
}

// ─── Content type override ──────────────────────────────────────────────────

#[test]
fn content_type_override_forces_engine() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<span>42</span>"#.to_string());

    // Force regex on HTML content
    let results = cs.query(-1, r"regex:(\d+)", Some(ContentType::Html));
    assert_eq!(results, vec!["42"]);
}

// ─── Content management ────────────────────────────────────────────────────

#[test]
fn content_count() {
    let mut cs = ChadSelect::new();
    assert_eq!(cs.content_count(), 0);

    cs.add_text("a".to_string());
    cs.add_html("<b>b</b>".to_string());
    cs.add_json(r#"{"c": 3}"#.to_string());
    assert_eq!(cs.content_count(), 3);
}

#[test]
fn clear_removes_all() {
    let mut cs = ChadSelect::new();
    cs.add_text("a".to_string());
    cs.add_text("b".to_string());
    cs.clear();
    assert_eq!(cs.content_count(), 0);

    let results = cs.query(-1, "regex:.", None);
    assert!(results.is_empty());
}

// ─── Mixed content multi-engine ─────────────────────────────────────────────

#[test]
fn mixed_content_regex_spans_all() {
    let mut cs = ChadSelect::new();
    cs.add_text("id=100".to_string());
    cs.add_html("<div>id=200</div>".to_string());
    cs.add_json(r#"{"id": "id=300"}"#.to_string());

    let results = cs.query(-1, r"regex:id=(\d+)", None);
    assert_eq!(results, vec!["100", "200", "300"]);
}

#[test]
fn css_only_hits_html() {
    let mut cs = ChadSelect::new();
    cs.add_text("not html".to_string());
    cs.add_json(r#"{"x": 1}"#.to_string());
    cs.add_html("<span class='x'>found</span>".to_string());

    let results = cs.query(-1, "css:.x", None);
    assert_eq!(results, vec!["found"]);
}

#[test]
fn json_only_hits_json() {
    let mut cs = ChadSelect::new();
    cs.add_text("not json".to_string());
    cs.add_html("<div>not json</div>".to_string());
    cs.add_json(r#"{"key": "found"}"#.to_string());

    let result = cs.select(0, "json:key");
    assert_eq!(result, "found");
}

// ─── Empty content ──────────────────────────────────────────────────────────

#[test]
fn query_on_empty_returns_empty() {
    let cs = ChadSelect::new();
    let results = cs.query(-1, "regex:anything", None);
    assert!(results.is_empty());
}

// ─── Delimiter safety ───────────────────────────────────────────────────────

#[test]
fn pipe_in_xpath_not_confused_with_functions() {
    let mut cs = ChadSelect::new();
    cs.add_html(
        r#"
        <span class="a">Alpha</span>
        <span class="b">Beta</span>
    "#
        .to_string(),
    );

    // XPath union uses `|` — should NOT be interpreted as a function pipe.
    let results = cs.query(-1, "xpath://span[@class='a']/text() | //span[@class='b']/text()", None);
    assert_eq!(results.len(), 2);
    assert!(results.contains(&"Alpha".to_string()));
    assert!(results.contains(&"Beta".to_string()));
}

#[test]
fn double_arrow_pipe_works_with_xpath_union() {
    let mut cs = ChadSelect::new();
    cs.add_html(
        r#"
        <span class="a">  Alpha  </span>
        <span class="b">  Beta  </span>
    "#
        .to_string(),
    );

    // XPath union + `>>` function pipe — both should work simultaneously.
    // NOTE: the union `|` is inside the XPath expression, and `>>` separates functions.
    // Currently split_functions splits on the first `>>`, so the xpath part keeps the `|`.
    let results = cs.query(
        -1,
        "xpath://span[@class='a']/text() | //span[@class='b']/text() >> normalize-space()",
        None,
    );
    // Both results should be normalize-spaced.
    for r in &results {
        assert!(!r.starts_with(' '));
        assert!(!r.ends_with(' '));
    }
}
