//! ChadSelect Rust — standalone benchmark.
//!
//! Uses the **same fixtures and queries** as `chadselect-py/tests/bench.py`
//! so results are directly comparable.
//!
//! Run with: `cargo bench`

use chadselect::ChadSelect;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

// ═══════════════════════════════════════════════════════════════════════════════
//  Fixtures — identical to chadselect-py/tests/bench.py
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
        html.push_str(&format!(
            "    <span class=\"ticker\">{} {}%</span>\n", sym, pct
        ));
    }
    html.push_str("  </div>\n</article>\n</body></html>");
    html
}

/// 200-item JSON API response.
fn api_json() -> String {
    let mut items = Vec::with_capacity(200);
    for i in 0..200 {
        items.push(format!(
            concat!(
                "{{\"id\":{i},\"name\":\"Product {i}\",\"price\":{price}.99,",
                "\"in_stock\":{stock},",
                "\"categories\":[\"{cat1}\",\"{cat2}\"],",
                "\"specs\":{{\"weight\":\"{w}kg\",\"color\":\"{color}\"}}}}"
            ),
            i = i,
            price = 10 + i * 3,
            stock = if i % 3 == 0 { "false" } else { "true" },
            cat1 = ["tools", "electronics", "home", "garden", "toys"][i % 5],
            cat2 = ["home", "office", "outdoor", "kitchen", "garage"][i % 5],
            w = format!("{}.{}", 1 + i % 10, i % 10),
            color = COLORS[i % 5],
        ));
    }
    format!("{{\"api_version\":\"2.1\",\"products\":[{}]}}", items.join(","))
}

// ═══════════════════════════════════════════════════════════════════════════════
//  CSS benchmarks (6 tasks)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_css(c: &mut Criterion) {
    let ecom = ecommerce_html();
    let news = news_html();

    let mut g = c.benchmark_group("css");

    g.bench_function("product_titles", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "css:.title")));
    });

    g.bench_function("current_prices", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "css:.price.current")));
    });

    g.bench_function("buy_link_hrefs", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "css:a.buy >> get-attr('href')")));
    });

    g.bench_function("ticker_symbols", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(news.clone());
        b.iter(|| black_box(cs.query(-1, "css:.ticker")));
    });

    g.bench_function("first_vin_with_chain", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(0, "css:.vin >> substring-after('VIN: ') >> uppercase()")));
    });

    g.bench_function("all_data_sku_attrs", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "css:.product >> get-attr('data-sku')")));
    });

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  XPath benchmarks (6 tasks)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_xpath(c: &mut Criterion) {
    let ecom = ecommerce_html();
    let news = news_html();

    let mut g = c.benchmark_group("xpath");

    g.bench_function("current_prices", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "xpath://span[@class='price current']/text()")));
    });

    g.bench_function("buy_link_hrefs", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "xpath://a[@class='buy']/@href")));
    });

    g.bench_function("headline", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(news.clone());
        b.iter(|| black_box(cs.select(0, "xpath://h1[@class='headline']/text()")));
    });

    g.bench_function("all_data_skus", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "xpath://div[@class='product']/@data-sku")));
    });

    g.bench_function("union_h1_h2", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, "xpath://h1/text() | //h2/text()")));
    });

    g.bench_function("normalize_space", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(news.clone());
        b.iter(|| black_box(cs.select(0, "xpath:normalize-space(//h1)")));
    });

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Regex benchmarks (4 tasks)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_regex(c: &mut Criterion) {
    let ecom = ecommerce_html();
    let news = news_html();

    let mut g = c.benchmark_group("regex");

    g.bench_function("dollar_prices", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, r"regex:\$[\d,]+\.\d{2}")));
    });

    g.bench_function("vin_numbers", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, r"regex:VIN:\s*([\w]+)")));
    });

    g.bench_function("data_sku_capture", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(ecom.clone());
        b.iter(|| black_box(cs.query(-1, r#"regex:data-sku="(SKU-\d+)""#)));
    });

    g.bench_function("pct_changes", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(news.clone());
        b.iter(|| black_box(cs.query(-1, r"regex:[+-]\d+\.\d+%")));
    });

    g.finish();
}

// ═══════════════════════════════════════════════════════════════════════════════
//  JMESPath benchmarks (4 tasks)
// ═══════════════════════════════════════════════════════════════════════════════

fn bench_json(c: &mut Criterion) {
    let json = api_json();

    let mut g = c.benchmark_group("json");

    g.bench_function("product_names", |b| {
        let mut cs = ChadSelect::new();
        cs.add_json(json.clone());
        b.iter(|| black_box(cs.query(-1, "json:products[].name")));
    });

    g.bench_function("in_stock_filter", |b| {
        let mut cs = ChadSelect::new();
        cs.add_json(json.clone());
        b.iter(|| black_box(cs.query(-1, "json:products[?in_stock].name")));
    });

    g.bench_function("nested_spec", |b| {
        let mut cs = ChadSelect::new();
        cs.add_json(json.clone());
        b.iter(|| black_box(cs.select(0, "json:products[0].specs.weight")));
    });

    g.bench_function("flatten_categories", |b| {
        let mut cs = ChadSelect::new();
        cs.add_json(json.clone());
        b.iter(|| black_box(cs.query(-1, "json:products[].categories[]")));
    });

    g.finish();
}

criterion_group!(benches, bench_css, bench_xpath, bench_regex, bench_json);
criterion_main!(benches);
