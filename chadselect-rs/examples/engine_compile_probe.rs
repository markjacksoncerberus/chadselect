//! Per-VDP compile-cost probe for the regex / jmespath / css engines.
//!
//! Profiling the live `hermes_rust` fleet showed ~28% of CPU in regex
//! *compilation* (`build_many_from_hir`) and ~13% in jmespath compilation —
//! because `regex::Regex::new`, `jmespath::compile`, and `Selector::parse` are
//! called fresh on **every** `select()`/`query()`, i.e. recompiled for every
//! page. The selectors are static; only the document changes. The XPath engine
//! already caches its compiled `Transform` (see `xpath_eval::COMPILED`); these
//! three never got the same treatment.
//!
//! This probe reproduces the production access pattern and measures the cost
//! the cache removes: it replays the real fleet selectors (`eval/selectors.json`)
//! the way a crawler does — a **fresh `ChadSelect` per page** (so nothing is
//! shared via the per-document content cache), each selector compiled + run
//! once per page, across many pages. With a process-wide compiled-pattern cache,
//! page 1 pays compilation and pages 2..N hit the cache — exactly the fleet's
//! steady state (same selectors, page after page).
//!
//! Run BEFORE and AFTER adding the caches; compare `per page` and `pages/sec`.
//!
//! ```text
//! cargo run --release --example engine_compile_probe
//! cargo run --release --example engine_compile_probe -- --pages 60
//! ```

use std::time::Instant;

use chadselect::ChadSelect;
use serde_json::Value;

const SELECTORS_JSON: &str = include_str!("../eval/selectors.json");

fn load(engine: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(SELECTORS_JSON).unwrap();
    v.as_array()
        .unwrap()
        .iter()
        .filter(|r| r["engine"] == engine)
        .filter_map(|r| r["selector"].as_str().map(String::from))
        .collect()
}

/// A compact but realistic VDP-ish HTML page (~30 KB) — regex/css run against
/// this. Compile cost is independent of document size; a modest page keeps the
/// match cost from masking what we're measuring.
fn vdp_html() -> String {
    let mut s = String::with_capacity(32_000);
    s.push_str("<!doctype html><html><head><title>2021 Honda Accord</title></head><body>");
    for i in 0..60 {
        s.push_str(&format!(
            "<div class=\"spec-info specifics-label\" data-vin=\"1HGCM{i:05}\" data-name=\"Vehicle {i}\">\
               <span class=\"info__label\">Mileage:</span><span class=\"value\">{m} mi</span>\
               <span class=\"info__label\">Engine:</span><span class=\"value\">2.0L Turbo</span>\
               <a href=\"tel:555{i:07}\">Call</a><a href=\"mailto:s{i}@x.com\">Email</a>\
               <p>VIN: 1HGCM{i:05} Stock: STK{i} Price: ${p}</p>\
             </div>",
            m = 10000 + i * 137,
            p = 20000 + i,
        ));
    }
    s.push_str(
        "<script type=\"application/ld+json\">{\"@context\":\"https://schema.org\",\
         \"@graph\":[{\"@type\":\"Vehicle\",\"vehicleIdentificationNumber\":\"1HGCM00001\",\
         \"bodyType\":\"Sedan\",\"brand\":{\"name\":\"Honda\",\"address\":{\"addressLocality\":\"Reno\",\
         \"addressRegion\":\"NV\",\"postalCode\":\"89501\"}},\"offers\":{\"price\":24999,\
         \"seller\":{\"telephone\":\"555-0100\"}}}]}</script>",
    );
    s.push_str("</body></html>");
    s
}

/// The ld+json payload a `json:` selector targets.
fn vdp_json() -> String {
    String::from(
        "{\"@context\":\"https://schema.org\",\"@graph\":[{\"@type\":\"Vehicle\",\
         \"vehicleIdentificationNumber\":\"1HGCM00001\",\"bodyType\":\"Sedan\",\"model\":\"Accord\",\
         \"brand\":{\"name\":\"Honda\",\"address\":{\"addressLocality\":\"Reno\",\"addressRegion\":\"NV\",\
         \"postalCode\":\"89501\"}},\"offers\":{\"price\":24999,\"seller\":{\"telephone\":\"555-0100\"}}}]}",
    )
}

/// Replay one engine's selectors across `pages` fresh documents (fresh
/// `ChadSelect` each page → no cross-page sharing except a process-wide compiled
/// cache, if present). Returns (total_seconds, selectors_per_page).
fn replay(engine: &str, selectors: &[String], pages: usize, html: &str, json: &str) -> (f64, usize) {
    let prefixed: Vec<String> = selectors.iter().map(|s| format!("{engine}:{s}")).collect();
    let t = Instant::now();
    for _ in 0..pages {
        // Fresh per page — this is the crawler's reality (one ChadSelect per VDP).
        let mut cs = ChadSelect::new();
        if engine == "json" {
            cs.add_json(json.to_string());
        } else {
            cs.add_html(html.to_string());
        }
        for q in &prefixed {
            std::hint::black_box(cs.query(-1, q));
        }
    }
    (t.elapsed().as_secs_f64(), prefixed.len())
}

fn main() {
    let mut pages = 40usize;
    let mut a = std::env::args().skip(1);
    while let Some(x) = a.next() {
        if x == "--pages" {
            pages = a.next().and_then(|s| s.parse().ok()).unwrap_or(pages);
        }
    }

    let html = vdp_html();
    let json = vdp_json();
    let regex = load("regex");
    let css = load("css");
    let jsonp = load("json");

    println!("\n══ engine compile-cost probe — {pages} pages (fresh ChadSelect each) ══");
    println!("content: {} B HTML / {} B JSON\n", html.len(), json.len());
    println!(
        " {:<8} {:>6}  {:>11}  {:>12}  {:>12}",
        "engine", "sel", "total", "per page", "per sel/page"
    );

    let mut grand = 0.0;
    for (eng, sels) in [("regex", &regex), ("css", &css), ("json", &jsonp)] {
        let (secs, n) = replay(eng, sels, pages, &html, &json);
        grand += secs;
        let per_page_ms = secs * 1e3 / pages as f64;
        let per_sel_us = secs * 1e6 / (pages * n.max(1)) as f64;
        println!(
            " {:<8} {:>6}  {:>9.3} s  {:>9.3} ms  {:>9.2} µs",
            eng, n, secs, per_page_ms, per_sel_us
        );
    }
    println!("\n total wall: {:.3} s over {} pages ({:.1} ms/page)\n", grand, pages, grand * 1e3 / pages as f64);
}
