//! Allocation + CPU harness for the JMESPath (`json:`) engine.
//!
//! ## Why this exists
//! Consumer allocation profiling of a single VDP extraction attributed **64.5%
//! of all allocations** (~248k) to the `json:` engine — far more than XPath.
//! The tell: the top sites were `jmespath::variable::serialize_str`,
//! `Box::new(RcInner<Variable>)` and `BTreeNode<String, Rc<Variable>>`. That is
//! the signature of converting the *entire* JSON document into jmespath's
//! `Variable` tree on **every** `json:` query.
//!
//! Mechanism: `Expression::search(&serde_json::Value)` runs `data.to_jmespath()`
//! on each call. On stable (no nightly `specialized` feature) that dispatches to
//! `impl<T: Serialize> ToJmespath` → `Variable::from_serializable`, a full serde
//! walk that re-allocates the whole `Rc<Variable>` tree. chadselect caches the
//! `serde_json::Value` (saving only the parse), not the converted tree — so a
//! vehicle that runs N `json:` selectors pays the conversion N times.
//!
//! This harness measures the steady-state cost of running a realistic set of
//! `json:` selectors against one already-parsed document, in two dimensions:
//!   * **allocations** (count + bytes) via a counting `#[global_allocator]`;
//!   * **CPU** (wall-clock over many iterations).
//! Run: `cargo test --release --test jmespath_alloc report -- --nocapture --test-threads=1`

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use chadselect::ChadSelect;

// ── Counting allocator (count + bytes) ───────────────────────────────────────

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

// ── Realistic fixture: a VDP-shaped embedded JSON document ───────────────────
//
// Mirrors the kind of `application/ld+json` / `__INITIAL_STATE__` blob a vehicle
// detail page embeds: a vehicle object with scalar fields, a specs array of
// {name,value} objects, a features string array, and a nested dealer object.

fn vdp_json(specs: usize) -> String {
    let mut s = String::with_capacity(specs * 80 + 512);
    s.push_str(r#"{"vehicle":{"vin":"5LM5J7XC4MGL00000","year":2025,"make":"Lincoln","model":"Aviator","trim":"Reserve","price":78250,"mileage":12,"exterior":"Infinite Black","interior":"Ebony","drivetrain":"AWD","engine":"3.0L V6","transmission":"10-Speed Automatic","mpgCity":18,"mpgHwy":26,"stock":"LA12345","specs":["#);
    for i in 0..specs {
        if i > 0 {
            s.push(',');
        }
        s.push_str(r#"{"name":"Feature "#);
        s.push_str(&i.to_string());
        s.push_str(r#"","value":"Value "#);
        s.push_str(&i.to_string());
        s.push_str(r#"","category":"Comfort"}"#);
    }
    s.push_str(r#"],"features":["#);
    for i in 0..specs {
        if i > 0 {
            s.push(',');
        }
        s.push_str(r#""Feature string "#);
        s.push_str(&i.to_string());
        s.push('"');
    }
    s.push_str(r#"]},"dealer":{"name":"Cerberus Lincoln","city":"Austin","state":"TX","zip":"78701","phone":"512-555-0100","rating":4.8}}"#);
    s
}

// A realistic field-extraction selector set (~24 `json:` queries per vehicle).
const QUERIES: &[&str] = &[
    "json:vehicle.vin",
    "json:vehicle.year",
    "json:vehicle.make",
    "json:vehicle.model",
    "json:vehicle.trim",
    "json:vehicle.price",
    "json:vehicle.mileage",
    "json:vehicle.exterior",
    "json:vehicle.interior",
    "json:vehicle.drivetrain",
    "json:vehicle.engine",
    "json:vehicle.transmission",
    "json:vehicle.mpgCity",
    "json:vehicle.mpgHwy",
    "json:vehicle.stock",
    "json:vehicle.specs[*].name",
    "json:vehicle.specs[*].value",
    "json:vehicle.specs[0].category",
    "json:vehicle.features",
    "json:vehicle.features[0]",
    "json:dealer.name",
    "json:dealer.city",
    "json:dealer.state",
    "json:dealer.rating",
];

/// Count allocations for running the full selector set once against an
/// already-parsed document with warm compile caches — the steady-state per-VDP
/// `json:` cost, not one-time parse/compile setup.
fn extraction_allocs(json: &str) -> (u64, u64) {
    let _lock = SERIAL.lock().unwrap();
    let mut cs = ChadSelect::new();
    cs.add_json(json.to_string());
    // Warm: force the serde parse AND compile every expression once.
    for q in QUERIES {
        let _ = cs.query(-1, q);
    }
    let (c0, b0) = snap();
    for q in QUERIES {
        let _ = cs.query(-1, q);
    }
    let (c1, b1) = snap();
    (c1 - c0, b1 - b0)
}

/// Wall-clock for `iters` full selector-set passes against one parsed document.
///
/// Holds `SERIAL` for its whole body: the counting allocator is process-global,
/// so this must not run concurrently with `extraction_allocs` or its allocations
/// would inflate that test's snapshot delta.
fn extraction_cpu(json: &str, iters: u32) -> std::time::Duration {
    let _lock = SERIAL.lock().unwrap();
    let mut cs = ChadSelect::new();
    cs.add_json(json.to_string());
    for q in QUERIES {
        let _ = cs.query(-1, q); // warm
    }
    let t = Instant::now();
    for _ in 0..iters {
        for q in QUERIES {
            let _ = cs.query(-1, q);
        }
    }
    t.elapsed()
}

/// Guard: a full `json:` extraction pass must not re-convert the whole document
/// per query. With the per-document `Rcvar` cache, allocations are roughly
/// constant in document size; without it they scale with `specs`. We assert the
/// *slope* (Δallocs / Δspecs) is small — a per-query whole-doc conversion blows
/// it up (the document has O(specs) nodes, converted once per query × |QUERIES|).
const MAX_ALLOCS_PER_SPEC: f64 = 12.0;

#[test]
fn json_extraction_allocs_do_not_scale_with_doc_size() {
    // Warm shared lazy statics so the first measured doc isn't charged setup.
    let _ = extraction_allocs(&vdp_json(16));

    let (a_lo, _) = extraction_allocs(&vdp_json(50));
    let (a_hi, _) = extraction_allocs(&vdp_json(200));
    let per_spec = (a_hi as f64 - a_lo as f64) / (200.0 - 50.0);
    eprintln!(
        "[jmespath-alloc-guard] {a_lo} allocs @50 specs, {a_hi} @200 specs \
         → {per_spec:.1} allocs/spec over {} queries (ceiling {MAX_ALLOCS_PER_SPEC})",
        QUERIES.len()
    );
    assert!(
        per_spec < MAX_ALLOCS_PER_SPEC,
        "json: extraction made {per_spec:.1} allocations per spec-row of document \
         growth — the per-query whole-document Variable-conversion signature. The \
         parsed tree must be converted once per document and cached, not per query."
    );
}

/// Diagnostic report (not a guard). Run with:
/// `cargo test --release --test jmespath_alloc report -- --nocapture --test-threads=1`
#[test]
fn report() {
    eprintln!("\n──────── json: extraction cost vs document size ({} queries/pass) ────────", QUERIES.len());
    eprintln!("  {:<10} {:>10} {:>10} {:>12} {:>14}", "specs", "allocs", "KiB", "allocs/query", "µs/pass");
    eprintln!("  {}", "─".repeat(60));
    // Warm shared statics.
    let _ = extraction_allocs(&vdp_json(16));
    for &specs in &[10usize, 50, 100, 200, 400] {
        let json = vdp_json(specs);
        let (allocs, bytes) = extraction_allocs(&json);
        let iters = 200;
        let dur = extraction_cpu(&json, iters);
        let us_per_pass = dur.as_secs_f64() * 1e6 / iters as f64;
        eprintln!(
            "  {:<10} {:>10} {:>10} {:>12.1} {:>14.1}",
            specs,
            allocs,
            bytes / 1024,
            allocs as f64 / QUERIES.len() as f64,
            us_per_pass
        );
    }
    eprintln!();
}
