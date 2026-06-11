//! Per-query allocation-count guard for the engines, with an emphasis on XPath.
//!
//! ## Why this exists
//! `memory_harness` measures *retained bytes* with *non-matching* selectors — by
//! design it isolates parse/DOM cost and never exercises the per-query
//! result-collection path. But the field incident was **allocation churn on the
//! matching path**: a consumer's `extract_vehicle_data` over a 580 KiB VDP made
//! ~425k short-lived allocations, ~90% of them from the XPath engine, which
//! residency profiling can't see (near-zero RSS) yet which pegs the kernel
//! servicing alloc/free traffic under a decaying allocator + concurrency.
//!
//! Root cause was structural churn in the XPath evaluator (chadpath): the path
//! `compose` deep-cloned the whole evaluation `Context` once per step, predicate
//! filtering re-cloned it per context node, and the `ENode` adapter boxed every
//! axis iterator. A single `//a[…]//b[…]/text()` over a few-hundred-node page
//! made tens of thousands of those clones. The fixes (in-place context
//! evolution, `ctxt.clone()` over `ContextBuilder::from`, per-item context-Vec
//! reuse, cached boolean comparison results, and concrete un-boxed axis
//! iterators) cut the per-matched-row allocation count of that shape by ~45%.
//!
//! This test owns its own counting `#[global_allocator]`, so it lives in its own
//! integration-test binary. It runs in two modes:
//!   * always: a **guard** asserting XPath stays under a per-row alloc ceiling;
//!   * `--nocapture`: prints a per-engine attribution table and a size-scaling
//!     table (run `-- --nocapture --test-threads=1`).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use chadselect::ChadSelect;

// ── Counting allocator (count + bytes; mirrors the consumer's guard) ─────────

struct CountingAlloc;
static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size.saturating_sub(layout.size()) as u64, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;
static SERIAL: Mutex<()> = Mutex::new(());

fn snap() -> (u64, u64) {
    (COUNT.load(Ordering::Relaxed), BYTES.load(Ordering::Relaxed))
}

// ── Realistic fixture: a VDP-shaped product page with N spec rows ────────────

fn page(rows: usize) -> String {
    let mut s = String::with_capacity(rows * 200 + 256);
    s.push_str("<html><body><div class=\"vdp\">");
    s.push_str("<h1 class=\"vehicle-title\">2025 Lincoln Aviator</h1>");
    for i in 0..rows {
        s.push_str("<li class=\"spec-item\"><span class=\"label\">Feature ");
        s.push_str(&i.to_string());
        s.push_str("</span><span class=\"value\">Value ");
        s.push_str(&i.to_string());
        s.push_str("</span></li>");
    }
    s.push_str("</div></body></html>");
    s
}

/// Count allocations for issuing `query` once against an already-parsed doc with
/// warm compile caches — i.e. the steady-state per-VDP cost, not one-time setup.
fn count_query(html: &str, warm: &str, query: &str) -> (u64, u64, usize) {
    let _lock = SERIAL.lock().unwrap();
    let mut cs = ChadSelect::new();
    cs.add_html(html.to_string());
    let _ = cs.query(-1, warm); // force DOM parse
    let _ = cs.query(-1, query); // warm the per-expression compile cache
    let (c0, b0) = snap();
    let n = cs.query(-1, query).len();
    let (c1, b1) = snap();
    (c1 - c0, b1 - b0, n)
}

const XPATH_PRED: &str =
    "xpath://li[contains(@class,'spec-item')]//span[@class='value']/text()";

/// Per matched row, the worst realistic XPath shape (`//a[pred]//b[pred]/text()`)
/// allocates this many times after the fixes. Pre-fix it was ~141/row. The
/// ceiling catches a regression of the per-node churn (which scales with
/// `rows × predicate-complexity`) while tolerating noise and minor changes.
const MAX_XPATH_ALLOCS_PER_ROW: f64 = 95.0;

#[test]
fn xpath_allocations_per_row_are_bounded() {
    // Warm the lazy statics shared across documents (selector/xpath/regex caches,
    // qualname interner) so the first measured doc isn't charged one-time setup.
    let warm = page(16);
    let _ = count_query(&warm, "css:li", XPATH_PRED);

    // Measure at two sizes and use the *slope* (Δallocs / Δrows): this cancels
    // any fixed per-query overhead and isolates the per-row churn the incident
    // was about. A per-node re-walk regression blows the slope past the ceiling.
    let (a_lo, _, n_lo) = count_query(&page(100), "css:li", XPATH_PRED);
    let (a_hi, _, n_hi) = count_query(&page(400), "css:li", XPATH_PRED);
    assert!(n_lo == 100 && n_hi == 400, "fixture/selector mismatch: {n_lo}, {n_hi}");

    let per_row = (a_hi - a_lo) as f64 / (n_hi - n_lo) as f64;
    eprintln!(
        "[xpath-alloc-guard] {a_lo} allocs @100 rows, {a_hi} @400 rows \
         → {per_row:.1} allocs/row (ceiling {MAX_XPATH_ALLOCS_PER_ROW})"
    );
    assert!(
        per_row < MAX_XPATH_ALLOCS_PER_ROW,
        "XPath made {per_row:.1} allocations per matched row (ceiling \
         {MAX_XPATH_ALLOCS_PER_ROW}). This is the per-node evaluation-churn \
         signature (allocs scaling with rows × predicate complexity). See the \
         module doc."
    );
}

/// Diagnostic report (not a guard). Run with:
/// `cargo test --release --test alloc_attribution report -- --nocapture --test-threads=1`
#[test]
fn report() {
    let html = page(200);
    // Per-engine attribution: same doc, one matching query of each engine type.
    let cases: &[(&str, &str, &str)] = &[
        ("css broad+text", "css:li", "css:li.spec-item span.value >> normalize-space()"),
        ("css attr select", "css:li", "css:span.value"),
        ("xpath //pred text", "css:li", XPATH_PRED),
        ("xpath //attr", "css:li", "xpath://span[@class='value']"),
        ("xpath //*", "css:li", "xpath://*"),
        ("regex over raw", "css:li", r#"regex:Value (\d+)"#),
    ];
    eprintln!("\n──────── per-query allocation attribution ({} KiB doc, 200 rows) ────────", html.len() / 1024);
    eprintln!("  {:<20} {:>10} {:>10} {:>8}", "engine / query", "allocs", "KiB", "matches");
    eprintln!("  {}", "─".repeat(52));
    for (label, warm, q) in cases {
        let (allocs, bytes, n) = count_query(&html, warm, q);
        eprintln!("  {:<20} {:>10} {:>10} {:>8}", label, allocs, bytes / 1024, n);
    }

    eprintln!("\n  XPath //-pred alloc scaling vs document size:");
    eprintln!("  {:<10} {:>10} {:>12} {:>10}", "rows", "allocs", "allocs/row", "matches");
    eprintln!("  {}", "─".repeat(46));
    for &rows in &[50usize, 100, 200, 400] {
        let (allocs, _b, n) = count_query(&page(rows), "css:li", XPATH_PRED);
        eprintln!("  {:<10} {:>10} {:>12.1} {:>10}", rows, allocs, allocs as f64 / rows as f64, n);
    }
    eprintln!();
}
