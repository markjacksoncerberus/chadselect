//! Performance regression guards for the XPath engine.
//!
//! The 0.3.x overhaul introduced an **O(n²)** predicate-filter blowup: chadpath's
//! `ContextBuilder::from(&Context)` deep-cloned the entire context (every node in
//! the context + `current` sequences) once per context node in `step_predicated`
//! / `filter_seq`. A `//tag[@attr=…]` over an N-node page did ~N² clones —
//! warm eval of `//span[@class='price current']/text()` over a 200-product page
//! went from ~3 ms to ~200 ms, pegging the crawler fleet at 100% CPU. Fixed by
//! cloning the eval context once and mutating only the focus fields per node.
//!
//! These guards lock that in. Rather than assert wall-clock (machine-dependent),
//! they assert the **scaling ratio**: time at 4× document size / time at 1×.
//! For O(n) that ratio is ~4; for O(n²) it is ~16. A ceiling of 9 cleanly
//! separates the two while tolerating timing noise. The check is therefore
//! valid in debug or release, on fast or slow hardware.
//!
//! Timing tests are `#[ignore]` by default. Run explicitly (release strongly
//! recommended — debug eval is ~10× slower and makes the run sluggish):
//!
//! ```text
//! cargo test --release --test xpath_perf_guard -- --ignored --nocapture
//! ```

use std::time::{Duration, Instant};

use chadselect::ChadSelect;

/// Build a product-listing page with `n` product blocks (the shape the crawler
/// fleet actually hits: many siblings, each with class-attributed children).
fn page(n: usize) -> String {
    let mut s = String::with_capacity(n * 220 + 64);
    s.push_str("<html><body><div class=\"products\">");
    for i in 0..n {
        s.push_str(&format!(
            concat!(
                "<div class=\"product\" data-sku=\"SKU-{i}\">",
                "<h2 class=\"title\">Product {i}</h2>",
                "<span class=\"price original\">${o}.99</span>",
                "<span class=\"price current\">${c}.99</span>",
                "<a class=\"buy\" href=\"/buy/{i}\">Buy</a>",
                "</div>"
            ),
            i = i,
            o = 100 + i,
            c = 80 + i,
        ));
    }
    s.push_str("</div></body></html>");
    s
}

/// Median-of-3 per-iteration time for `expr` over `html`. The document is parsed
/// once (cached) and warmed before timing, so this measures *evaluation*.
fn per_query(html: &str, expr: &str, iters: usize) -> Duration {
    let mut cs = ChadSelect::new();
    cs.add_html(html.to_string());
    let _ = cs.query(-1, expr); // warm: parse + first eval (caches the DOM)

    let mut samples = [Duration::MAX; 3];
    for s in samples.iter_mut() {
        let t = Instant::now();
        for _ in 0..iters {
            let _ = cs.query(-1, expr);
        }
        *s = t.elapsed() / iters as u32;
    }
    samples.sort();
    samples[1] // median
}

/// Assert evaluation of `expr` scales sub-quadratically: 4× the document must
/// cost well under 4²× the time. Returns the observed ratio for logging.
fn assert_subquadratic(expr: &str) -> f64 {
    const SMALL: usize = 500;
    const BIG: usize = 2000; // 4× SMALL
    const RATIO_CEIL: f64 = 9.0; // linear≈4, quadratic≈16

    let ts = per_query(&page(SMALL), expr, 4);
    let tb = per_query(&page(BIG), expr, 4);

    // Guard against dividing two sub-100µs noise blobs into a bogus ratio.
    assert!(
        ts >= Duration::from_micros(50),
        "{expr}: small-size sample {ts:?} too small to time reliably"
    );

    let ratio = tb.as_secs_f64() / ts.as_secs_f64();
    println!("  {expr}\n    {SMALL}→{ts:?}  {BIG}→{tb:?}  ratio={ratio:.2} (ceil {RATIO_CEIL})");
    assert!(
        ratio < RATIO_CEIL,
        "{expr}: 4× document size → {ratio:.1}× eval time (≥{RATIO_CEIL} ⇒ super-linear \
         regression; the O(n²) predicate-filter blowup is back). small={ts:?} big={tb:?}"
    );
    ratio
}

// ─────────────────────────────────────────────────────────────────────────────
//  Scaling guards — one per query shape that exercises predicate filtering.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[ignore = "timing guard; run with --release --ignored --nocapture"]
fn attr_predicate_scales_linearly() {
    // The exact shape that regressed.
    assert_subquadratic("xpath://span[@class='price current']/text()");
}

#[test]
#[ignore = "timing guard; run with --release --ignored --nocapture"]
fn attr_predicate_with_attr_axis_scales_linearly() {
    assert_subquadratic("xpath://div[@class='product']/@data-sku");
    assert_subquadratic("xpath://a[@class='buy']/@href");
}

#[test]
#[ignore = "timing guard; run with --release --ignored --nocapture"]
fn wildcard_predicate_scales_linearly() {
    // Worst case: the predicate is tested against *every* element.
    assert_subquadratic("xpath://*[@class='price current']");
}

#[test]
#[ignore = "timing guard; run with --release --ignored --nocapture"]
fn nested_and_positional_predicates_scale_linearly() {
    assert_subquadratic("xpath://div[@class='product'][1]/h2/text()");
    assert_subquadratic("xpath://div[span[@class='price current']]/@data-sku");
}

#[test]
#[ignore = "timing guard; run with --release --ignored --nocapture"]
fn non_predicate_paths_stay_linear() {
    // Controls: these never used the buggy path; they should also stay linear.
    assert_subquadratic("xpath://span/text()");
    assert_subquadratic("xpath://h1/text() | //h2/text()");
}

// ─────────────────────────────────────────────────────────────────────────────
//  Absolute backstop — a large page must complete quickly in wall-clock terms.
//  Pre-fix, this query over 5000 products took *minutes*; post-fix it is well
//  under a second in release. The ceiling is deliberately generous so it only
//  fires on a true complexity regression, not on a slow CI box.
// ─────────────────────────────────────────────────────────────────────────────

#[test]
#[ignore = "timing guard; run with --release --ignored --nocapture"]
fn large_page_predicate_query_completes_quickly() {
    let html = page(5000);
    let mut cs = ChadSelect::new();
    cs.add_html(html);

    let t = Instant::now();
    let got = cs.query(-1, "xpath://span[@class='price current']/text()");
    let elapsed = t.elapsed();

    assert_eq!(got.len(), 5000, "must still return every match");
    println!("  5000-product predicate query: {elapsed:?}");
    assert!(
        elapsed < Duration::from_secs(5),
        "5000-product predicate query took {elapsed:?} (>5s ⇒ the O(n²) blowup is back)"
    );
}
