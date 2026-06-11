//! Regression tests for the XPath numeric / relational / boolean / string-length
//! conformance fixes (2026-06 correctness pass on the `chadpath` fork).
//!
//! Each of these was a real, observed bug surfaced while building the robustness
//! suite. They are kept as standing regression guards. Fixes landed in the fork:
//!   * relational `<,>,<=,>=` on node values → numeric (was inverted +
//!     lexicographic): `general_comparison` + `Item::compare` operand order.
//!   * `number(node)` / `sum(node-set)` → use the node's string-value
//!     (`Item::to_double` no longer returns NaN for nodes).
//!   * top-level boolean results serialize as "true"/"false"
//!     (`ValueData` `Display` gained its missing `Boolean` arm).
//!   * `string-length()` implemented (`Transform::StringLength`).

use chadselect::ChadSelect;

const HTML: &str = r#"<html><body>
  <ul>
    <li data-n="1" data-p="10.5">alpha</li>
    <li data-n="2" data-p="20.0">beta</li>
    <li data-n="3" data-p="30.5">gamma</li>
    <li data-n="4" data-p="40.0">delta</li>
  </ul>
  <div class="price">$1,299.00</div>
</body></html>"#;

fn q(expr: &str) -> Vec<String> {
    let mut c = ChadSelect::new();
    c.add_html(HTML.to_string());
    c.query(-1, expr)
}

// ── Relational comparison on node values is numeric and correctly oriented ──

#[test]
fn relational_gt_ge_lt_le_on_attribute() {
    assert_eq!(q("xpath://li[@data-n>'2']/text()"), ["gamma", "delta"]);
    assert_eq!(q("xpath://li[@data-n>=3]/text()"), ["gamma", "delta"]);
    assert_eq!(q("xpath://li[@data-n<2]/text()"), ["alpha"]);
    assert_eq!(q("xpath://li[@data-n<=2]/text()"), ["alpha", "beta"]);
}

#[test]
fn relational_is_numeric_not_lexicographic() {
    // "10" < "9" lexically, but 10 > 9 numerically — proves true numeric compare.
    let html = r#"<html><body><ul>
        <li n="9">nine</li><li n="10">ten</li></ul></body></html>"#;
    let mut c = ChadSelect::new();
    c.add_html(html.to_string());
    assert_eq!(c.query(-1, "xpath://li[@n>9]/text()"), ["ten"]);
    assert_eq!(c.query(-1, "xpath://li[@n>=10]/text()"), ["ten"]);
}

#[test]
fn relational_on_float_attribute() {
    assert_eq!(q("xpath://li[@data-p>25]/text()"), ["gamma", "delta"]);
}

// ── number() / sum() over node string-values ──

#[test]
fn number_of_node() {
    assert_eq!(q("xpath:number(//li[1]/@data-n)"), ["1"]);
}

#[test]
fn sum_over_nodeset() {
    assert_eq!(q("xpath:sum(//li/@data-n)"), ["10"]); // 1+2+3+4
    assert_eq!(q("xpath:sum(//li/@data-p)"), ["101"]); // 10.5+20+30.5+40
}

// ── Top-level boolean-valued expressions serialize as true/false ──

#[test]
fn top_level_booleans() {
    assert_eq!(q("xpath:boolean(//li[1])"), ["true"]);
    assert_eq!(q("xpath:contains('abc','b')"), ["true"]);
    assert_eq!(q("xpath:starts-with('alpha','al')"), ["true"]);
    assert_eq!(q("xpath:not(//li[@data-n='99'])"), ["true"]);
    assert_eq!(q("xpath:true()"), ["true"]);
    assert_eq!(q("xpath:false()"), ["false"]);
}

#[test]
fn top_level_comparison_is_boolean() {
    assert_eq!(q("xpath:count(//li) > 3"), ["true"]);
    assert_eq!(q("xpath:count(//li) > 99"), ["false"]);
}

// ── string-length() ──

#[test]
fn string_length() {
    assert_eq!(q("xpath:string-length('hello')"), ["5"]);
    assert_eq!(
        q("xpath:string-length(normalize-space(//div[@class='price']))"),
        ["9"] // "$1,299.00"
    );
    assert_eq!(q("xpath:string-length(//li[1])"), ["5"]); // "alpha"
}
