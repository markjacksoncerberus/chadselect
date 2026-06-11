//! XPath engine **head-to-head**: current `chadpath` (xrust fork over the shared
//! `scraper`/html5ever DOM) vs the retired `sxd` stack (`sxd_html` +
//! `sxd-xpath`, the v0.2.x backend).
//!
//! Motivation: after the 0.3.x XPath overhaul, production memory dropped but
//! fleet CPU pegged at ~100% (was ~10%). This bench isolates *where* the CPU
//! goes so the regression can be attributed instead of guessed at.
//!
//! For each query we measure two scenarios:
//!   * **warm** — document parsed once, evaluation only. Isolates the engine's
//!     per-query evaluation CPU (the steady-state cost when one selector runs
//!     across many already-loaded docs).
//!   * **cold** — parse + evaluate every iteration. This is what the crawler
//!     fleet actually pays per page.
//!
//! Plus two diagnostic micro-groups:
//!   * `parse_only` — html5ever (scraper) vs sxd_html parse cost alone.
//!   * `chadpath_overhead` — `ENode::root_of` (the per-query document-order
//!     map rebuild) vs a full evaluate, to size the adapter tax.
//!
//! Run with: `cargo bench --bench xpath_compare`
//! Narrow to one engine/scenario with e.g.
//! `cargo bench --bench xpath_compare -- warm/`

use std::rc::Rc;

use chadselect::engine::xnode::ENode;
use chadselect::engine::xpath_eval;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use scraper::Html;

use sxd_xpath::{Context, Factory, Value as XPathValue};

// ═══════════════════════════════════════════════════════════════════════════════
//  Fixtures — identical to engine_bench.rs / chadselect-py/tests/bench.py
// ═══════════════════════════════════════════════════════════════════════════════

const COLORS: [&str; 5] = ["Black", "White", "Silver", "Red", "Blue"];
const ENGINES: [&str; 5] = ["2.0L Turbo", "3.6L V6", "5.0L V8", "Electric", "Hybrid"];

/// 200-product e-commerce page (~30 KB).
fn ecommerce_html() -> String {
    let mut html = String::with_capacity(40_000);
    html.push_str("<html><body>\n<div class=\"products\">\n");
    for i in 0..200 {
        html.push_str(&format!(
            concat!(
                "  <div class=\"product\" data-sku=\"SKU-{i:03}\">\n",
                "    <h2 class=\"title\">Product {i}</h2>\n",
                "    <span class=\"price original\">${orig}.99</span>\n",
                "    <span class=\"price current\">${cur}.99</span>\n",
                "    <span class=\"vin\">VIN: 1HGCM{i:05}A{extra:06}</span>\n",
                "    <div class=\"details\">\n",
                "      <div class=\"spec\"><span class=\"label\">Color:</span> <span class=\"value\">{color}</span></div>\n",
                "      <div class=\"spec\"><span class=\"label\">Engine:</span> <span class=\"value\">{engine}</span></div>\n",
                "    </div>\n",
                "    <a class=\"buy\" href=\"/buy/{i}\">Buy Now</a>\n",
                "  </div>\n",
            ),
            i = i,
            orig = 100 + i * 5,
            cur = 80 + i * 4,
            extra = 100_000 + i * 7,
            color = COLORS[i % 5],
            engine = ENGINES[i % 5],
        ));
    }
    html.push_str("</div>\n</body></html>");
    html
}

/// News page with tickers.
fn news_html() -> String {
    let mut html = String::from(concat!(
        "<html><body>\n",
        "<article class=\"story\">\n",
        "  <h1 class=\"headline\">Markets Rally on Fed Decision</h1>\n",
        "  <div class=\"byline\">By Jane Smith | 2025-12-15</div>\n",
        "  <p class=\"summary\">Major indices climbed as the Federal Reserve held rates steady.</p>\n",
        "  <div class=\"tickers\">\n",
    ));
    let tickers = [
        ("AAPL", "+2.3"), ("GOOGL", "+1.8"), ("MSFT", "+3.1"), ("AMZN", "-0.4"),
        ("META", "+1.2"), ("NVDA", "+4.5"), ("TSLA", "-1.7"), ("JPM", "+0.9"),
        ("BAC", "+0.3"), ("WMT", "-0.2"), ("DIS", "+1.1"), ("NFLX", "+2.8"),
        ("AMD", "+3.4"), ("INTC", "-0.8"), ("PYPL", "+0.6"), ("SQ", "+1.9"),
        ("UBER", "+0.4"), ("LYFT", "-1.3"), ("SNAP", "+0.7"), ("PINS", "-0.5"),
    ];
    for (sym, pct) in &tickers {
        html.push_str(&format!("    <span class=\"ticker\">{} {}%</span>\n", sym, pct));
    }
    html.push_str("  </div>\n</article>\n</body></html>");
    html
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Query catalog — the 6 XPath tasks, with which fixture each runs on.
// ═══════════════════════════════════════════════════════════════════════════════

enum Fix {
    Ecom,
    News,
}

struct Task {
    name: &'static str,
    expr: &'static str,
    fixture: Fix,
}

const TASKS: &[Task] = &[
    Task { name: "current_prices",  expr: "//span[@class='price current']/text()", fixture: Fix::Ecom },
    Task { name: "buy_link_hrefs",  expr: "//a[@class='buy']/@href",               fixture: Fix::Ecom },
    Task { name: "all_data_skus",   expr: "//div[@class='product']/@data-sku",     fixture: Fix::Ecom },
    Task { name: "union_h1_h2",     expr: "//h1/text() | //h2/text()",             fixture: Fix::Ecom },
    Task { name: "headline",        expr: "//h1[@class='headline']/text()",        fixture: Fix::News },
    Task { name: "normalize_space", expr: "normalize-space(//h1)",                 fixture: Fix::News },
];

fn fixture_html(f: &Fix) -> String {
    match f {
        Fix::Ecom => ecommerce_html(),
        Fix::News => news_html(),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  sxd (old engine) helpers — replicate the retired v0.2.x evaluation path.
// ═══════════════════════════════════════════════════════════════════════════════

/// Mirror of the old `xpath_value_to_strings`: flatten a result to trimmed,
/// non-empty strings in document order. Kept so sxd does the *same* output work
/// chadpath does (fair comparison — neither gets to skip result materialization).
fn sxd_value_to_strings(result: &XPathValue) -> Vec<String> {
    match result {
        XPathValue::Nodeset(ns) => ns
            .document_order()
            .into_iter()
            .map(|n| n.string_value())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        XPathValue::String(s) => {
            let t = s.trim();
            if t.is_empty() { vec![] } else { vec![t.to_string()] }
        }
        XPathValue::Number(n) => vec![n.to_string()],
        XPathValue::Boolean(b) => vec![b.to_string()],
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
//  WARM: evaluation only, document parsed once. Isolates engine eval CPU.
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_warm(c: &mut Criterion) {
    let mut g = c.benchmark_group("warm");

    for task in TASKS {
        let html = fixture_html(&task.fixture);

        // ── chadpath: pre-parse the scraper DOM, evaluate repeatedly ──
        let doc = Rc::new(Html::parse_document(&html));
        g.bench_with_input(BenchmarkId::new("chadpath", task.name), task, |b, t| {
            b.iter(|| black_box(xpath_eval::evaluate(&doc, t.expr)));
        });

        // ── sxd: pre-parse the sxd document, pre-build factory/expr/context ──
        let package = sxd_html::parse_html(&html);
        let document = package.as_document();
        let factory = Factory::new();
        let expr = factory
            .build(task.expr)
            .expect("sxd build")
            .expect("sxd non-empty");
        let context = Context::new();
        g.bench_with_input(BenchmarkId::new("sxd", task.name), task, |b, _| {
            b.iter(|| {
                let v = expr.evaluate(&context, document.root()).expect("sxd eval");
                black_box(sxd_value_to_strings(&v))
            });
        });
    }

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  COLD: parse + evaluate every iteration. What the crawler fleet pays per page.
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_cold(c: &mut Criterion) {
    let mut g = c.benchmark_group("cold");

    for task in TASKS {
        let html = fixture_html(&task.fixture);

        g.bench_with_input(BenchmarkId::new("chadpath", task.name), task, |b, t| {
            b.iter(|| {
                let doc = Rc::new(Html::parse_document(&html));
                black_box(xpath_eval::evaluate(&doc, t.expr))
            });
        });

        g.bench_with_input(BenchmarkId::new("sxd", task.name), task, |b, t| {
            b.iter(|| {
                let package = sxd_html::parse_html(&html);
                let document = package.as_document();
                let factory = Factory::new();
                let expr = factory.build(t.expr).expect("sxd build").expect("non-empty");
                let context = Context::new();
                let v = expr.evaluate(&context, document.root()).expect("sxd eval");
                black_box(sxd_value_to_strings(&v))
            });
        });
    }

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  DIAGNOSTIC: parse cost alone — html5ever (scraper) vs sxd_html.
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_parse_only(c: &mut Criterion) {
    let ecom = ecommerce_html();
    let news = news_html();
    let mut g = c.benchmark_group("parse_only");

    for (name, html) in [("ecom", &ecom), ("news", &news)] {
        g.bench_with_input(BenchmarkId::new("chadpath_html5ever", name), html, |b, h| {
            b.iter(|| black_box(Html::parse_document(h)));
        });
        g.bench_with_input(BenchmarkId::new("sxd_html", name), html, |b, h| {
            b.iter(|| black_box(sxd_html::parse_html(h)));
        });
    }

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  DIAGNOSTIC: chadpath adapter tax — `ENode::root_of` rebuilds a full
//  document-order HashMap on *every* evaluate() call. This sizes that rebuild
//  against the whole evaluation, exposing it as a per-query fixed cost that
//  scales with document size (and that sxd's reusable document never pays).
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_chadpath_overhead(c: &mut Criterion) {
    let html = ecommerce_html();
    let doc = Rc::new(Html::parse_document(&html));
    let mut g = c.benchmark_group("chadpath_overhead");

    // The per-query document-order map rebuild (build_order over every node).
    g.bench_function("root_of_only", |b| {
        b.iter(|| black_box(ENode::root_of(&doc)));
    });

    // A full evaluate for reference (root_of is included inside it).
    g.bench_function("full_evaluate", |b| {
        b.iter(|| black_box(xpath_eval::evaluate(&doc, "//span[@class='price current']/text()")));
    });

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  DIAGNOSTIC: decompose `//span[@class='price current']/text()` to find which
//  piece of chadpath eval explodes. Same pre-parsed doc for every variant.
//    bare_descendant   — `//span`               : name-test only, no predicate
//    attr_predicate    — `//span[@class=...]`   : + attribute predicate
//    full              — `//span[@class=...]/text()` : + child text step
//    wildcard_pred     — `//*[@class=...]`       : predicate over *every* element
//  If bare is fast and attr_predicate is ~50×, the predicate path is the cause.
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_predicate_isolation(c: &mut Criterion) {
    let html = ecommerce_html();
    let chad = Rc::new(Html::parse_document(&html));

    let package = sxd_html::parse_html(&html);
    let document = package.as_document();
    let factory = Factory::new();
    let context = Context::new();

    let variants = [
        ("bare_descendant", "//span"),
        ("attr_predicate", "//span[@class='price current']"),
        ("full", "//span[@class='price current']/text()"),
        ("wildcard_pred", "//*[@class='price current']"),
    ];

    let mut g = c.benchmark_group("predicate_isolation");
    for (name, expr) in variants {
        g.bench_with_input(BenchmarkId::new("chadpath", name), expr, |b, e| {
            b.iter(|| black_box(xpath_eval::evaluate(&chad, e)));
        });
        let built = factory.build(expr).expect("build").expect("non-empty");
        g.bench_with_input(BenchmarkId::new("sxd", name), expr, |b, _| {
            b.iter(|| {
                let v = built.evaluate(&context, document.root()).expect("eval");
                black_box(sxd_value_to_strings(&v))
            });
        });
    }
    g.finish();
}

criterion_group!(
    benches,
    bench_warm,
    bench_cold,
    bench_parse_only,
    bench_chadpath_overhead,
    bench_predicate_isolation
);
criterion_main!(benches);
