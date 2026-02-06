//! Tests for the XPath 1.0 extraction engine.

use chadselect::ChadSelect;

const HTML: &str = r#"
<html>
    <body>
        <div class="container">
            <h1>Test Title</h1>
            <p>First paragraph</p>
            <p>Second paragraph</p>
            <span id="vin">1GCPAAEK7TZ152448</span>
            <span id="stock">TZ152448</span>
        </div>
    </body>
</html>
"#;

fn make_cs() -> ChadSelect {
    let mut cs = ChadSelect::new();
    cs.add_html(HTML.to_string());
    cs
}

// ─── Basic extraction ───────────────────────────────────────────────────────

#[test]
fn extract_text_by_tag() {
    let cs = make_cs();
    let result = cs.select(0, "xpath://h1/text()");
    assert_eq!(result, "Test Title");
}

#[test]
fn extract_text_by_id() {
    let cs = make_cs();
    let result = cs.select(0, "xpath://span[@id='vin']/text()");
    assert_eq!(result, "1GCPAAEK7TZ152448");
}

#[test]
fn extract_multiple_paragraphs() {
    let cs = make_cs();
    let results = cs.query(-1, "xpath://p/text()", None);
    assert_eq!(results, vec!["First paragraph", "Second paragraph"]);
}

// ─── XPath built-in functions ───────────────────────────────────────────────

#[test]
fn xpath_normalize_space_function() {
    let cs = make_cs();
    let result = cs.select(0, "xpath:normalize-space(//h1)");
    assert_eq!(result, "Test Title");
}

#[test]
fn xpath_string_function() {
    let cs = make_cs();
    let result = cs.select(0, "xpath:string(//span[@id='vin'])");
    assert_eq!(result, "1GCPAAEK7TZ152448");
}

// ─── Post-processing with >> ────────────────────────────────────────────────

#[test]
fn xpath_with_normalize_space_postprocess() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<span class="price">  $100  </span>"#.to_string());

    let results = cs.query(-1, "xpath://span[@class='price']/text() >> normalize-space()", None);
    assert_eq!(results, vec!["$100"]);
}

#[test]
fn xpath_with_substring_after() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<div class="vin">VIN: 1HGCM82633A123456</div>"#.to_string());

    let results = cs.query(-1, r#"xpath://div[@class='vin']/text() >> substring-after('VIN: ')"#, None);
    assert_eq!(results, vec!["1HGCM82633A123456"]);
}

#[test]
fn xpath_with_substring_before() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<div class="info">Price: $300</div>"#.to_string());

    let results = cs.query(-1, r#"xpath://div[@class='info']/text() >> substring-before(': ')"#, None);
    assert_eq!(results, vec!["Price"]);
}

#[test]
fn xpath_chained_functions() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<div class="vin">VIN: 1HGCM82633A123456</div>"#.to_string());

    let results = cs.query(
        -1,
        r#"xpath://div[@class='vin']/text() >> substring-after('VIN: ') >> substring(0, 3) >> uppercase()"#,
        None,
    );
    assert_eq!(results, vec!["1HG"]);
}

#[test]
fn xpath_normalize_space_vs_trim() {
    let mut cs = ChadSelect::new();
    cs.add_html(r#"<p class="description">  This is a great   vehicle!  </p>"#.to_string());

    let normalized = cs.query(-1, "xpath://p[@class='description']/text() >> normalize-space()", None);
    assert_eq!(normalized, vec!["This is a great vehicle!"]);

    let trimmed = cs.query(-1, "xpath://p[@class='description']/text() >> trim()", None);
    assert_eq!(trimmed, vec!["This is a great   vehicle!"]);
}

// ─── Error handling ─────────────────────────────────────────────────────────

#[test]
fn invalid_xpath_returns_empty() {
    let cs = make_cs();
    let results = cs.query(-1, "xpath:[[[invalid", None);
    assert!(results.is_empty());
}

#[test]
fn xpath_no_match_returns_empty() {
    let cs = make_cs();
    let results = cs.query(-1, "xpath://nonexistent/text()", None);
    assert!(results.is_empty());
}

// ─── XPath union operator ───────────────────────────────────────────────────

#[test]
fn xpath_union_operator_works_without_functions() {
    let cs = make_cs();
    // The XPath `|` union operator should work because we use `>>` for functions.
    let results = cs.query(-1, "xpath://span[@id='vin']/text() | //span[@id='stock']/text()", None);
    assert_eq!(results.len(), 2);
    assert!(results.contains(&"1GCPAAEK7TZ152448".to_string()));
    assert!(results.contains(&"TZ152448".to_string()));
}

// ─── Index selection ────────────────────────────────────────────────────────

#[test]
fn xpath_index_selection() {
    let cs = make_cs();

    let first = cs.query(0, "xpath://p/text()", None);
    assert_eq!(first, vec!["First paragraph"]);

    let second = cs.query(1, "xpath://p/text()", None);
    assert_eq!(second, vec!["Second paragraph"]);
}
