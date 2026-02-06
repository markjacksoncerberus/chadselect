//! Benchmarks comparing extraction speed across all four engines
//! on a realistically sized HTML document.
//!
//! Run with: `cargo bench`

use chadselect::ChadSelect;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Generate a chunky HTML page (~50KB) with structured, repeated data.
fn big_html() -> String {
    let mut html = String::with_capacity(60_000);
    html.push_str(r#"<!DOCTYPE html><html><head><title>Inventory</title></head><body>"#);
    html.push_str(r#"<div id="inventory">"#);

    for i in 0..200 {
        html.push_str(&format!(
            r#"<div class="vehicle" data-id="{id}">
  <h2 class="title">2024 Model-{id}</h2>
  <span class="price">${price}</span>
  <span class="vin">VIN: 1HGCM{id:05}A{extra:06}</span>
  <div class="details">
    <div class="item"><span class="label">Mileage:</span> <span class="value">{miles} mi</span></div>
    <div class="item"><span class="label">Color:</span> <span class="value">{color}</span></div>
    <div class="item"><span class="label">Engine:</span> <span class="value">{engine}</span></div>
  </div>
  <a class="link" href="/vehicle/{id}">View Details</a>
</div>"#,
            id = i,
            price = 20_000 + i * 137,
            extra = 100_000 + i * 7,
            miles = 10_000 + i * 321,
            color = ["Black", "White", "Silver", "Red", "Blue"][i % 5],
            engine = ["2.0L Turbo", "3.6L V6", "5.0L V8", "Electric", "Hybrid"][i % 5],
        ));
    }

    html.push_str("</div></body></html>");
    html
}

/// Generate a JSON document with similar data.
fn big_json() -> String {
    let mut items = Vec::with_capacity(200);
    for i in 0..200 {
        items.push(format!(
            r#"{{"id":{id},"title":"2024 Model-{id}","price":{price},"vin":"1HGCM{id:05}A{extra:06}","mileage":{miles},"color":"{color}","engine":"{engine}"}}"#,
            id = i,
            price = 20_000 + i * 137,
            extra = 100_000 + i * 7,
            miles = 10_000 + i * 321,
            color = ["Black", "White", "Silver", "Red", "Blue"][i % 5],
            engine = ["2.0L Turbo", "3.6L V6", "5.0L V8", "Electric", "Hybrid"][i % 5],
        ));
    }
    format!(r#"{{"inventory":[{}]}}"#, items.join(","))
}

// ─── Benchmarks ─────────────────────────────────────────────────────────────

fn bench_first_match(c: &mut Criterion) {
    let html = big_html();
    let json = big_json();

    let mut group = c.benchmark_group("first_match");

    // ── Regex ───────────────────────────────────────────────────────────
    group.bench_function("regex", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(0, r"regex:VIN:\s*([\w]+)"));
        });
    });

    // ── CSS ─────────────────────────────────────────────────────────────
    group.bench_function("css", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(0, "css:.vin"));
        });
    });

    // ── XPath ───────────────────────────────────────────────────────────
    group.bench_function("xpath", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(0, "xpath://span[@class='vin']/text()"));
        });
    });

    // ── JMESPath ────────────────────────────────────────────────────────
    group.bench_function("jmespath", |b| {
        let mut cs = ChadSelect::new();
        cs.add_json(json.clone());
        b.iter(|| {
            black_box(cs.select(0, "json:inventory[0].vin"));
        });
    });

    group.finish();
}

fn bench_all_matches(c: &mut Criterion) {
    let html = big_html();
    let json = big_json();

    let mut group = c.benchmark_group("all_200_matches");

    group.bench_function("regex", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.query(-1, r"regex:VIN:\s*([\w]+)"));
        });
    });

    group.bench_function("css", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.query(-1, "css:.vin"));
        });
    });

    group.bench_function("xpath", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.query(-1, "xpath://span[@class='vin']/text()"));
        });
    });

    group.bench_function("jmespath", |b| {
        let mut cs = ChadSelect::new();
        cs.add_json(json.clone());
        b.iter(|| {
            black_box(cs.query(-1, "json:inventory[].vin"));
        });
    });

    group.finish();
}

fn bench_with_post_processing(c: &mut Criterion) {
    let html = big_html();

    let mut group = c.benchmark_group("first_match_with_functions");

    group.bench_function("css + normalize-space", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(0, "css:.vin >> normalize-space()"));
        });
    });

    group.bench_function("css + substring-after + uppercase", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(0, "css:.vin >> substring-after('VIN: ') >> uppercase()"));
        });
    });

    group.bench_function("xpath + substring-after + uppercase", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(
                0,
                "xpath://span[@class='vin']/text() >> substring-after('VIN: ') >> uppercase()",
            ));
        });
    });

    group.bench_function("regex (no post-processing needed)", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select(0, r"regex:VIN:\s*([\w]+)"));
        });
    });

    group.finish();
}

fn bench_select_first_fallback(c: &mut Criterion) {
    let html = big_html();

    let mut group = c.benchmark_group("select_first_fallback");

    group.bench_function("miss_miss_hit (3 queries)", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select_first(vec![
                (0, "css:#nonexistent"),
                (0, "xpath://div[@id='nope']/text()"),
                (0, "css:.vin"),
            ]));
        });
    });

    group.bench_function("hit_on_first (1 query)", |b| {
        let mut cs = ChadSelect::new();
        cs.add_html(html.clone());
        b.iter(|| {
            black_box(cs.select_first(vec![
                (0, "css:.vin"),
                (0, "xpath://span[@class='vin']/text()"),
            ]));
        });
    });

    group.finish();
}

fn bench_scaling(c: &mut Criterion) {
    let html = big_html();

    let mut group = c.benchmark_group("css_index_scaling");

    for &idx in &[0i32, 49, 99, 199] {
        group.bench_with_input(BenchmarkId::from_parameter(idx), &idx, |b, &idx| {
            let mut cs = ChadSelect::new();
            cs.add_html(html.clone());
            b.iter(|| {
                black_box(cs.select(idx, "css:.price"));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_first_match,
    bench_all_matches,
    bench_with_post_processing,
    bench_select_first_fallback,
    bench_scaling,
);
criterion_main!(benches);
