//! Comprehensive XPath 1.0 robustness suite for the `chadpath`-backed engine.
//!
//! This is the correctness net for the engine: a broad sweep of axes, node
//! tests, predicates (positional + per-parent), the common function library,
//! unions, attribute handling, entity decoding, document-order guarantees, and
//! resilience to adversarial/malformed HTML. Every expected value here was
//! captured from the live engine and cross-checked against XPath 1.0 semantics.
//!
//! Companion suites:
//!   * `xpath_perf_guard.rs`  — asserts evaluation stays ~linear (no O(n²)).
//!   * `xpath_known_gaps.rs`  — documents known conformance gaps (#[ignore]d).

use chadselect::ChadSelect;

const HTML: &str = r#"<html><body>
  <div id="main" class="container">
    <h1>Catalog</h1>
    <ul class="list">
      <li class="item first" data-n="1">alpha</li>
      <li class="item" data-n="2">beta</li>
      <li class="item special" data-n="3">gamma</li>
      <li class="item" data-n="4">delta</li>
    </ul>
    <table>
      <tr><th>Key</th><th>Val</th></tr>
      <tr><td>VIN</td><td>ABC123</td></tr>
      <tr><td>Mileage</td><td>3,500</td></tr>
    </table>
    <p class="note">Hello &amp; welcome &lt;friend&gt;</p>
    <div class="price">   $1,299.00   </div>
    <a href="/buy/1" class="buy">Buy 1</a>
    <a href="/buy/2" class="buy">Buy 2</a>
    <!-- a comment -->
    <span class="empty"></span>
    <section><article><p>deep para</p></article></section>
  </div>
</body></html>"#;

fn cs() -> ChadSelect {
    let mut c = ChadSelect::new();
    c.add_html(HTML.to_string());
    c
}

/// Run an xpath query, returning all matches.
fn q(expr: &str) -> Vec<String> {
    cs().query(-1, expr)
}

/// Assert a query yields exactly `expected`, in order.
fn eq(expr: &str, expected: &[&str]) {
    assert_eq!(q(expr), expected, "query: {expr}");
}

// ─────────────────────────────────────────────────────────────────────────────
//  Axes
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn axis_child_and_descendant() {
    eq("xpath:/html/body/div/h1/text()", &["Catalog"]);
    eq("xpath://ul/li/text()", &["alpha", "beta", "gamma", "delta"]);
    eq(
        "xpath://div[@id='main']/descendant::li/text()",
        &["alpha", "beta", "gamma", "delta"],
    );
    // // is descendant-or-self; nested deep content reachable in one hop.
    eq("xpath://section//p/text()", &["deep para"]);
}

#[test]
fn axis_wildcard_child() {
    eq("xpath://ul/*/text()", &["alpha", "beta", "gamma", "delta"]);
    eq("xpath://*[@id='main']/h1/text()", &["Catalog"]);
}

#[test]
fn axis_parent_and_self() {
    // `..` parent abbreviation
    eq("xpath://li[@data-n='3']/../@class", &["list"]);
    eq("xpath://li[@data-n='2']/parent::ul/@class", &["list"]);
    // self::
    eq("xpath://li[@data-n='2']/self::node()/text()", &["beta"]);
    eq("xpath://li/self::li[@data-n='4']/text()", &["delta"]);
}

#[test]
fn axis_ancestor() {
    eq("xpath://li[@class='item first']/ancestor::div/@id", &["main"]);
    eq(
        "xpath://li[@class='item first']/ancestor-or-self::*/@class",
        &["container", "list", "item first"],
    );
}

#[test]
fn axis_siblings() {
    eq(
        "xpath://li[@data-n='2']/following-sibling::li/text()",
        &["gamma", "delta"],
    );
    eq(
        "xpath://li[@data-n='3']/preceding-sibling::li/text()",
        &["alpha", "beta"],
    );
    // following-sibling pinned by a value test on its neighbour
    eq(
        "xpath://td[.='VIN']/following-sibling::td/text()",
        &["ABC123"],
    );
}

#[test]
fn axis_following_preceding() {
    eq(
        "xpath://li[@data-n='2']/following::li/text()",
        &["gamma", "delta"],
    );
    eq(
        "xpath://li[@data-n='3']/preceding::li/text()",
        &["alpha", "beta"],
    );
}

// ─────────────────────────────────────────────────────────────────────────────
//  Node tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn node_test_text_and_comment() {
    eq("xpath://comment()", &["a comment"]);
    eq("xpath://div[@class='price']/text()", &["$1,299.00"]); // trimmed
}

#[test]
fn node_test_attribute() {
    eq("xpath://a[@class='buy']/@href", &["/buy/1", "/buy/2"]);
    eq("xpath://li/@data-n", &["1", "2", "3", "4"]);
    eq("xpath://li[last()]/@data-n", &["4"]);
    eq("xpath://a[2]/@href", &["/buy/2"]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Predicates — positional, per-parent, attribute, boolean-combination
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn predicate_positional() {
    eq("xpath://li[1]/text()", &["alpha"]);
    eq("xpath://li[2]/text()", &["beta"]);
    eq("xpath://li[last()]/text()", &["delta"]);
    eq("xpath://li[last()-1]/text()", &["gamma"]);
    eq("xpath://li[position()<3]/text()", &["alpha", "beta"]);
    eq("xpath://li[position()>=3]/text()", &["gamma", "delta"]);
    eq(
        "xpath://li[position()=2 or position()=3]/text()",
        &["beta", "gamma"],
    );
}

#[test]
fn predicate_per_parent_positional() {
    // The class of bug the CPU-fix branch also hardened: a positional predicate
    // on a step must apply PER PARENT, not over the flattened set.
    eq("xpath://tr/td[1]/text()", &["VIN", "Mileage"]);
    eq("xpath://table//td[1]/text()", &["VIN", "Mileage"]);
    eq("xpath://tr[2]/td[2]/text()", &["ABC123"]);
}

#[test]
fn predicate_attribute() {
    eq("xpath://li[@data-n='2']/text()", &["beta"]);
    eq("xpath://li[@data-n]/text()", &["alpha", "beta", "gamma", "delta"]);
    eq(
        "xpath://li[@class and @data-n]/text()",
        &["alpha", "beta", "gamma", "delta"],
    );
    eq("xpath://li[@class='item']/text()", &["beta", "delta"]); // exact class match
}

#[test]
fn predicate_multi_and_nested() {
    // Two exact-'item' lis (beta, delta); [1] is per-parent → beta.
    eq("xpath://ul/li[@class='item'][1]/text()", &["beta"]);
    // Nested predicate: ul that CONTAINS an li with data-n=3.
    eq("xpath://ul[li[@data-n='3']]/@class", &["list"]);
}

#[test]
fn predicate_boolean_logic() {
    eq(
        "xpath://li[@data-n='1' or @data-n='4']/text()",
        &["alpha", "delta"],
    );
    eq("xpath://li[not(@data-n='2')]/text()", &["alpha", "gamma", "delta"]);
    eq("xpath://li[@data-n!='2']/text()", &["alpha", "gamma", "delta"]);
}

#[test]
fn predicate_with_string_functions() {
    eq("xpath://li[contains(@class,'special')]/text()", &["gamma"]);
    eq("xpath://li[starts-with(text(),'be')]/text()", &["beta"]);
    eq("xpath://p[contains(@class,'note')]/text()", &["Hello & welcome <friend>"]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Function library (string / numeric aggregates / names)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn fn_string_value_and_normalize() {
    eq("xpath:string(//li[1])", &["alpha"]);
    eq("xpath:normalize-space(//div[@class='price'])", &["$1,299.00"]);
    eq("xpath:normalize-space(//p[@class='note'])", &["Hello & welcome <friend>"]);
}

#[test]
fn fn_substring_family() {
    eq("xpath:substring(//li[1]/text(),1,3)", &["alp"]);
    eq("xpath:substring-before(//div[@class='price'],'.')", &["$1,299"]);
    eq("xpath:substring-after(//div[@class='price'],'$')", &["1,299.00"]);
    eq("xpath:concat(//li[1]/text(),'-',//li[4]/text())", &["alpha-delta"]);
    eq("xpath:translate(//li[1]/text(),'al','AL')", &["ALphA"]);
}

#[test]
fn fn_count() {
    eq("xpath:count(//li)", &["4"]);
    eq("xpath:count(//li[@class='item'])", &["2"]);
    eq("xpath:count(//tr)", &["3"]);
}

#[test]
fn fn_names() {
    eq("xpath:name(//div[@id='main'])", &["div"]);
    eq("xpath:local-name(//li[1])", &["li"]);
}

#[test]
fn fn_numeric_scalar() {
    eq("xpath:floor(3.7)", &["3"]);
    eq("xpath:ceiling(3.2)", &["4"]);
    eq("xpath:round(3.5)", &["4"]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Unions, entities, ordering, empties
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn union_returns_both_in_document_order() {
    eq(
        "xpath://h1/text() | //p[@class='note']/text()",
        &["Catalog", "Hello & welcome <friend>"],
    );
}

#[test]
fn entities_are_decoded() {
    eq("xpath://p[@class='note']/text()", &["Hello & welcome <friend>"]);
}

#[test]
fn empty_and_no_match_yield_empty() {
    assert!(q("xpath://nonexistent/text()").is_empty());
    assert!(q("xpath://li[@data-n='999']/text()").is_empty());
    assert!(q("xpath://span[@class='empty']/text()").is_empty()); // empty element
    assert!(q("xpath://li[@class='item']/@nope").is_empty()); // missing attr
}

#[test]
fn results_are_trimmed_and_blank_filtered() {
    // Surrounding whitespace on the price text node is trimmed; blanks dropped.
    eq("xpath://div[@class='price']/text()", &["$1,299.00"]);
}

// ─────────────────────────────────────────────────────────────────────────────
//  Resilience — malformed / adversarial HTML must never panic
// ─────────────────────────────────────────────────────────────────────────────

fn run_on(html: &str, expr: &str) -> Vec<String> {
    let mut c = ChadSelect::new();
    c.add_html(html.to_string());
    c.query(-1, expr)
}

#[test]
fn malformed_html_does_not_panic() {
    // Each of these is pathological HTML the html5ever tree-builder must coerce;
    // the engine must return *something* (often non-empty) and never panic.
    let cases = [
        "<html><body><div><span>x</span>",                 // unclosed tags
        "<p><b><i>text</p></b></i>",                        // misnested formatting
        "<table><td>orphan cell</td></table>",              // broken table
        "<div><div><div><div>deep</div></div></div></div>", // nesting
        "<ul><li>a<li>b<li>c</ul>",                         // implied </li>
        "<!-- unterminated comment <div>x</div>",           // comment soup
        "",                                                 // empty document
        "<html>&amp;&lt;&gt;&#65;",                         // entity-only body
    ];
    for html in cases {
        // Mix of selectors, including ones that route through the deep-stack path.
        for expr in ["xpath://div/text()", "xpath://li/text()", "xpath://*[@class]"] {
            let _ = run_on(html, expr); // must not panic
        }
    }
}

#[test]
fn deeply_nested_expression_is_handled() {
    // Long/deeply-nested expressions are routed to a large-stack thread rather
    // than overflowing; must not panic and must still evaluate.
    let html = "<html><body><div class='x'><span>hit</span></div></body></html>";
    let deep = format!("xpath://div[{}@class='x'{}]/span/text()", "(".repeat(40), ")".repeat(40));
    let _ = run_on(html, &deep); // no panic / no overflow
    // A plainly-correct nested predicate still works.
    eq("xpath://ul[li[@data-n='3']]/@class", &["list"]);
}

#[test]
fn large_document_correctness() {
    // Scale check for *correctness* (perf is guarded separately): a big page
    // must still return every match in order with the right count.
    let mut html = String::from("<html><body>");
    for i in 0..1000 {
        html.push_str(&format!("<div class='p'><span class='price'>${i}</span></div>"));
    }
    html.push_str("</body></html>");
    let got = run_on(&html, "xpath://span[@class='price']/text()");
    assert_eq!(got.len(), 1000);
    assert_eq!(got[0], "$0");
    assert_eq!(got[999], "$999");
}
