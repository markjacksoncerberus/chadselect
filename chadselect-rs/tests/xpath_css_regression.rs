//! Regression coverage for the consumer-reported XPath/CSS defects.
//! A: positional predicates (fixed 0.3.1). D/E: CSS combinators after a text
//! pseudo + exact-label→sibling (fixed). C: xrust per-step positional on
//! non-child axes is still broken upstream; the parenthesized `(...)[N]`
//! workaround is asserted so we notice if either changes.
use chadselect::ChadSelect;

fn doc() -> ChadSelect {
    let mut c = ChadSelect::new();
    c.add_html(r#"<html><body>
        <table><tr><td>VIN</td><td>ABC123</td></tr></table>
        <div id="sp"><span>VIN</span><span>WP0AB2A90SS225386</span><span>extra</span></div>
        <div class="r"><span>Exterior Color</span><span>Black</span></div>
        <div class="r"><span>OEM Exterior Color</span><span>Super Black</span></div>
    </body></html>"#.to_string());
    c
}

#[test]
fn bug_a_positional_regression() {
    let c = doc();
    assert_eq!(c.query(-1, "xpath://tr/td[1]/text()"), vec!["VIN"]);
    assert_eq!(c.query(-1, "xpath://tr/td[2]/text()"), vec!["ABC123"]);
    assert_eq!(c.query(-1, "xpath://div[@id='sp']/span[last()]/text()"), vec!["extra"]);
    assert_eq!(c.query(-1, "xpath://div[@id='sp']//*[2]/text()"), vec!["WP0AB2A90SS225386"]);
}

#[test]
fn bug_d_css_combinators_after_text_pseudo() {
    let c = doc();
    assert_eq!(c.query(-1, "css:span:has-text(VIN) + span"), vec!["WP0AB2A90SS225386"]);
    assert_eq!(c.query(-1, "css:span:has-text(VIN) ~ span"), vec!["WP0AB2A90SS225386", "extra"]);
    // descendant post still works
    assert_eq!(c.query(-1, "css:div:has-text(VIN) span"),
               vec!["VIN", "WP0AB2A90SS225386", "extra"]);
}

#[test]
fn bug_e_exact_label_then_sibling_value() {
    let c = doc();
    // exact label + sibling value — the faithful idiom, now expressible in CSS
    assert_eq!(c.query(-1, "css:span:text-equals(Exterior Color) + span"), vec!["Black"]);
}

#[test]
fn bug_c_following_sibling_positional_workaround() {
    let c = doc();
    // Per-step positional on following-sibling is broken upstream in xrust:
    assert_eq!(c.query(-1, "xpath://span[text()='VIN']/following-sibling::span[1]"),
               Vec::<String>::new(), "if this changes, xrust may have fixed BUG C");
    // The parenthesized node-set-index form is the correct workaround:
    assert_eq!(c.query(-1, "xpath:(//span[text()='VIN']/following-sibling::span)[1]"),
               vec!["WP0AB2A90SS225386"]);
}
