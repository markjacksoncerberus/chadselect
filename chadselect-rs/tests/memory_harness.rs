//! Memory-amplification harness for HTML parsing.
//!
//! Production crawlers feed ChadSelect arbitrary, often-malformed HTML. This
//! harness measures how much heap a given HTML body expands to **during** and
//! **after** parsing, across pathological input shapes, and reports the
//! amplification ratio (`bytes / input_len`).
//!
//! Measurement uses a process-global tracking allocator declared *only in this
//! test crate* — the published `chadselect` library and its consumers are
//! unaffected and gain no dependency.
//!
//! ## Running
//!
//! Report (prints the ratio table; never fails):
//! ```text
//! cargo test --test memory_harness report -- --nocapture --test-threads=1
//! ```
//!
//! Regression guards (assert amplification stays under threshold):
//! ```text
//! cargo test --test memory_harness guard -- --ignored --test-threads=1
//! ```
//!
//! `--test-threads=1` matters: the allocator counter is process-global, so
//! concurrent tests would corrupt each other's measurements. A serializing
//! mutex is also held during every measurement as a backstop.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use chadselect::ChadSelect;

// ═══════════════════════════════════════════════════════════════════════════
//  Tracking global allocator
// ═══════════════════════════════════════════════════════════════════════════

struct TrackingAlloc;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            let cur = CURRENT.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(cur, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = System.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            let old = layout.size();
            if new_size >= old {
                let cur = CURRENT.fetch_add(new_size - old, Ordering::Relaxed) + (new_size - old);
                PEAK.fetch_max(cur, Ordering::Relaxed);
            } else {
                CURRENT.fetch_sub(old - new_size, Ordering::Relaxed);
            }
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: TrackingAlloc = TrackingAlloc;

fn current() -> usize {
    CURRENT.load(Ordering::Relaxed)
}
fn peak() -> usize {
    PEAK.load(Ordering::Relaxed)
}
fn reset_peak() {
    PEAK.store(CURRENT.load(Ordering::Relaxed), Ordering::Relaxed);
}

/// Serializes measurements so parallel test threads can't corrupt the counter.
static SERIAL: Mutex<()> = Mutex::new(());

// ═══════════════════════════════════════════════════════════════════════════
//  Pathological HTML fixtures (each grows to ~`target` bytes)
// ═══════════════════════════════════════════════════════════════════════════

/// Append `unit` repeatedly until `buf` reaches `target` bytes.
fn fill(buf: &mut String, target: usize, unit: &str) {
    while buf.len() < target {
        buf.push_str(unit);
    }
}

/// Realistic, well-formed product blocks — the baseline denominator.
fn well_formed(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><div class=\"products\">");
    let mut i = 0usize;
    while s.len() < target {
        s.push_str("<div class=\"product\"><h2 class=\"title\">Product ");
        s.push_str(&i.to_string());
        s.push_str("</h2><span class=\"price\">$");
        s.push_str(&(100 + i).to_string());
        s.push_str(".99</span><a class=\"buy\" href=\"/buy/");
        s.push_str(&i.to_string());
        s.push_str("\">Buy</a></div>");
        i += 1;
    }
    s.push_str("</div></body></html>");
    s
}

/// Millions of tiny empty elements — maximizes DOM node count per byte.
fn tiny_repeated_tags(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body>");
    fill(&mut s, target, "<i></i>");
    s.push_str("</body></html>");
    s
}

/// One parent with a huge flat run of sibling elements.
fn wide_siblings(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><div>");
    fill(&mut s, target, "<span>x</span>");
    s.push_str("</div></body></html>");
    s
}

/// Deeply nested divs (then closed) — stresses tree depth and recursive drop.
fn deep_nesting(target: usize) -> String {
    let depth = (target / 11).max(1); // "<div>" + "</div>" ≈ 11 bytes/level
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body>");
    for _ in 0..depth {
        s.push_str("<div>");
    }
    s.push_str("leaf");
    for _ in 0..depth {
        s.push_str("</div>");
    }
    s.push_str("</body></html>");
    s
}

/// Misnested formatting elements — triggers the HTML5 adoption-agency
/// reconstruction. Prime suspect for super-linear blowup.
fn misnested_formatting(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><p>");
    fill(&mut s, target, "<b><i>");
    s.push_str("text</p></body></html>");
    s
}

/// Many never-closed block tags — forces open-element-stack growth.
fn unclosed_tags(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body>");
    fill(&mut s, target, "<div>");
    s.push_str("</body></html>");
    s
}

/// `<td>` cells with no row/section — exercises table foster-parenting.
fn broken_tables(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><table>");
    fill(&mut s, target, "<td>cell</td>");
    s.push_str("</table></body></html>");
    s
}

/// A single element carrying an enormous number of attributes.
fn many_attributes(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><div");
    let mut i = 0usize;
    while s.len() < target {
        s.push_str(" data-attr");
        s.push_str(&i.to_string());
        s.push_str("=\"v\"");
        i += 1;
    }
    s.push_str("></div></body></html>");
    s
}

/// A single attribute holding a gigantic value.
fn huge_attribute_value(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><div data-x=\"");
    fill(&mut s, target, "A");
    s.push_str("\"></div></body></html>");
    s
}

/// Comment-heavy soup — many comment nodes.
fn comment_heavy(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body>");
    fill(&mut s, target, "<!-- c -->");
    s.push_str("</body></html>");
    s
}

/// Entity-dense text — stresses the character-reference decoder.
fn entity_heavy(target: usize) -> String {
    let mut s = String::with_capacity(target + 64);
    s.push_str("<html><body><p>");
    fill(&mut s, target, "&amp;&lt;&gt;&#65;");
    s.push_str("</p></body></html>");
    s
}

// ═══════════════════════════════════════════════════════════════════════════
//  Measurement
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone)]
struct Report {
    pattern: &'static str,
    engine: &'static str,
    input_len: usize,
    /// Transient heap on top of the raw input string, observed during parse.
    parse_peak: usize,
    /// Heap retained on top of the raw input while the document is alive.
    dom_retained: usize,
    /// Heap still outstanding after dropping the document (logical leak).
    leaked: usize,
    millis: u128,
}

impl Report {
    /// Total peak footprint (raw input + transient) relative to input.
    fn peak_ratio(&self) -> f64 {
        (self.input_len + self.parse_peak) as f64 / self.input_len as f64
    }
    /// Total retained footprint (raw input + DOM) relative to input.
    fn retained_ratio(&self) -> f64 {
        (self.input_len + self.dom_retained) as f64 / self.input_len as f64
    }
}

/// Drive one fixture through one engine and capture allocation deltas.
///
/// `queries` are issued in order against the same document (so the "both" case
/// can hold two DOMs simultaneously). Selectors are chosen to match *nothing*,
/// isolating parse/DOM cost from result-collection cost.
fn measure(
    pattern: &'static str,
    engine: &'static str,
    gen: impl FnOnce() -> String,
    queries: &[&str],
) -> Report {
    let _lock = SERIAL.lock().unwrap();

    let html = gen();
    let input_len = html.len();

    let base = current();
    reset_peak();
    let t0 = Instant::now();

    let mut cs = ChadSelect::new();
    cs.add_html(html); // move — no new allocation
    for q in queries {
        let _ = cs.query(-1, q);
    }

    let millis = t0.elapsed().as_millis();
    let parse_peak = peak().saturating_sub(base);
    let dom_retained = current().saturating_sub(base);

    drop(cs);
    let leaked = current().saturating_sub(base);

    Report {
        pattern,
        engine,
        input_len,
        parse_peak,
        dom_retained,
        leaked,
        millis,
    }
}

type Gen = fn(usize) -> String;

/// All fixtures, ordered safest-first so partial output survives a crash.
fn fixtures() -> Vec<(&'static str, Gen)> {
    vec![
        ("well_formed", well_formed),
        ("tiny_repeated_tags", tiny_repeated_tags),
        ("wide_siblings", wide_siblings),
        ("broken_tables", broken_tables),
        ("comment_heavy", comment_heavy),
        ("entity_heavy", entity_heavy),
        ("huge_attribute_value", huge_attribute_value),
        ("many_attributes", many_attributes),
        ("unclosed_tags", unclosed_tags),
        ("misnested_formatting", misnested_formatting),
        ("deep_nesting", deep_nesting),
    ]
}

/// Parse a tiny document through every engine once, so lazy statics (selector,
/// xpath-factory, and regex caches) are initialized before the first real
/// measurement — otherwise that one-time setup is misattributed as a "leak".
fn warmup() {
    let _lock = SERIAL.lock().unwrap();
    let mut cs = ChadSelect::new();
    cs.add_html("<html><body><div class=\"x\">hi &amp; bye</div></body></html>".to_string());
    let _ = cs.query(-1, "css:div");
    let _ = cs.query(-1, "xpath://div");
    let _ = cs.query(-1, "css:div:has-text(hi)");
}

fn print_header(title: &str) {
    println!("\n{}", "═".repeat(96));
    println!("  {title}");
    println!("{}", "═".repeat(96));
    println!(
        "  {:<22} {:<7} {:>9} {:>11} {:>8} {:>11} {:>8} {:>7} {:>8}",
        "pattern", "engine", "input", "peak", "peak×", "retained", "ret×", "leak", "ms"
    );
    println!("  {}", "─".repeat(92));
}

fn print_row(r: &Report) {
    let flag = if r.peak_ratio() >= 20.0 || r.retained_ratio() >= 20.0 {
        " ⚠"
    } else {
        ""
    };
    println!(
        "  {:<22} {:<7} {:>9} {:>11} {:>7.1}x {:>11} {:>7.1}x {:>7} {:>8}{}",
        r.pattern,
        r.engine,
        human(r.input_len),
        human(r.input_len + r.parse_peak),
        r.peak_ratio(),
        human(r.input_len + r.dom_retained),
        r.retained_ratio(),
        human(r.leaked),
        r.millis,
        flag,
    );
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

fn human(bytes: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;
    if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0}KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  Report tests (always run; never assert)
// ═══════════════════════════════════════════════════════════════════════════

/// Patterns that detonate on the XPath path — either super-linear *time*
/// (HTML5/sxd reconstruction, deep open-element stacks) or extreme *memory*
/// (`entity_heavy` → 6500× on sxd). Kept on a tiny size ladder so the report
/// never hangs or OOMs; their cost is read from how `ms`/`peak×` scale.
const SUSPECT: [&str; 4] = [
    "unclosed_tags",
    "misnested_formatting",
    "deep_nesting",
    "entity_heavy",
];

fn is_suspect(name: &str) -> bool {
    SUSPECT.contains(&name)
}

/// Single serial report — everything runs in ONE test so plain
/// `cargo test --test memory_harness report -- --nocapture` is safe (no
/// parallel tests corrupting the process-global allocator counter).
///
/// `#[ignore]` so it doesn't slow ordinary `cargo test`. Run it with:
/// `cargo test --release --test memory_harness report -- --include-ignored --nocapture --test-threads=1`
#[test]
#[ignore = "diagnostic report; run explicitly with --include-ignored --nocapture"]
fn report() {
    warmup();
    // ── Suspect scaling ladder: watch ms & peak× grow with input ──
    print_header("SUPER-LINEAR SUSPECTS @ tiny ladder — watch ms & peak× vs input size");
    for (name, gen) in fixtures() {
        if !is_suspect(name) {
            continue;
        }
        for &size in &[8 * KB, 32 * KB, 128 * KB] {
            print_row(&measure(name, "css", || gen(size), &["css:zzz-nomatch"]));
            print_row(&measure(name, "xpath", || gen(size), &["xpath://zzz-nomatch"]));
        }
    }

    // ── Bulk (expected-linear) patterns. Single size: memory ratios are
    //    size-independent (verified), and the xpath path is O(n²) in time so
    //    larger inputs only add wall-clock, not insight. ──
    let size = 256 * KB;
    print_header(&format!("BULK patterns @ ~{} input", human(size)));
    for (name, gen) in fixtures() {
        if is_suspect(name) {
            continue;
        }
        print_row(&measure(name, "css", || gen(size), &["css:zzz-nomatch"]));
        print_row(&measure(name, "xpath", || gen(size), &["xpath://zzz-nomatch"]));
    }

    // ── Dual-DOM coexistence: same doc through BOTH engines, both DOMs cached ──
    print_header("DUAL-DOM: same doc queried by BOTH css and xpath @ ~256KB");
    for (name, gen) in fixtures() {
        if is_suspect(name) {
            continue;
        }
        print_row(&measure(
            name,
            "both",
            || gen(256 * KB),
            &["css:zzz-nomatch", "xpath://zzz-nomatch"],
        ));
    }

    // ── element_text_cache unbounded growth ──
    text_cache_growth(256 * KB);

    println!("\n  (⚠ = peak or retained ≥ 20× input)\n");
}

/// Issue many *distinct* text-pseudo base selectors against one document; the
/// `element_text_cache` is never evicted and clones on read, so retained heap
/// climbs with the number of distinct selectors.
fn text_cache_growth(size: usize) {
    let _lock = SERIAL.lock().unwrap();

    let html = well_formed(size);
    let input_len = html.len();

    let base = current();
    let mut cs = ChadSelect::new();
    cs.add_html(html);

    println!("\n{}", "═".repeat(96));
    println!(
        "  element_text_cache growth — distinct text-pseudo selectors on one {} doc",
        human(size)
    );
    println!("{}", "═".repeat(96));
    println!(
        "  {:<14} {:>11} {:>13} {:>9}",
        "selectors", "retained", "over-input", "ms"
    );
    println!("  {}", "─".repeat(50));

    let t0 = Instant::now();
    let mut issued = 0usize;
    for &cp in &[0usize, 50, 100, 200, 400, 800] {
        while issued < cp {
            let q = format!("css:div.product:nth-of-type({}):has-text(Product)", issued + 1);
            let _ = cs.query(-1, &q);
            issued += 1;
        }
        let retained = current().saturating_sub(base);
        println!(
            "  {:<14} {:>11} {:>13} {:>9}",
            issued,
            human(retained),
            human(retained.saturating_sub(input_len)),
            t0.elapsed().as_millis(),
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    drop(cs);
}

const KB: usize = 1024;

/// Largest input we'll feed a pattern in a guard without risking a hang.
/// Super-linear suspects stay tiny; linear patterns run at 1MB.
fn guard_size(name: &str) -> usize {
    if is_suspect(name) {
        128 * KB
    } else {
        // Kept at 512KB rather than 1MB: the xpath path is O(n²) in time, so
        // larger inputs only slow the guard without changing the (size-stable)
        // amplification ratios it checks.
        512 * KB
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  Guard tests (#[ignore] — run explicitly with --ignored)
// ═══════════════════════════════════════════════════════════════════════════

const LEAK_LIMIT_BYTES: usize = 64 * 1024;

/// Retained-amplification ceiling for **both** engines. Since the XPath engine
/// was moved onto the shared `scraper`/html5ever DOM, its retained footprint
/// equals the css path's; both amplify heavily but *bounded and linearly*
/// (ratio is size-independent). Worst observed is `misnested_formatting` at
/// ~93×; 120× leaves headroom to catch a genuine regression.
const RETAINED_LIMIT: f64 = 120.0;

/// **Invariant — must always pass.** No pattern leaks after the document is
/// dropped; parsing amplifies memory but does not retain it past `Drop`.
#[test]
#[ignore = "memory guard; run with --ignored --test-threads=1"]
fn guard_no_leaks_across_patterns() {
    warmup();
    for (pattern, gen) in fixtures() {
        let size = guard_size(pattern);
        for (engine, q) in [("css", "css:zzz-nomatch"), ("xpath", "xpath://zzz-nomatch")] {
            let r = measure(pattern, engine, || gen(size), &[q]);
            assert!(
                r.leaked <= LEAK_LIMIT_BYTES,
                "{pattern}/{engine} leaked {} after drop",
                human(r.leaked)
            );
        }
    }
}

/// **Invariant — must always pass.** Both engines stay under a bounded
/// *retained* amplification ceiling for every pattern (peak includes transient
/// evaluation working-set, which is bounded but noisier; retained is the
/// steady-state footprint that matters for a long-running crawler).
#[test]
#[ignore = "memory guard; run with --ignored --test-threads=1"]
fn guard_retained_is_bounded() {
    warmup();
    for (pattern, gen) in fixtures() {
        let size = guard_size(pattern);
        for (engine, q) in [("css", "css:zzz-nomatch"), ("xpath", "xpath://zzz-nomatch")] {
            let r = measure(pattern, engine, || gen(size), &[q]);
            assert!(
                r.retained_ratio() <= RETAINED_LIMIT,
                "{engine}/{pattern} retained {:.1}x exceeds {:.0}x ceiling",
                r.retained_ratio(),
                RETAINED_LIMIT
            );
        }
    }
}

/// **Regression guard for the fix.** The old `sxd_html` XPath backend blew up
/// quadratically — `entity_heavy` reached 6547× retained (1.6 GB from 256 KB).
/// The XPath engine now shares the html5ever DOM, so entity-dense input must
/// stay bounded and *not* grow with size. This locks in the fix.
#[test]
#[ignore = "memory guard; run with --ignored --test-threads=1"]
fn guard_xpath_entity_no_longer_explodes() {
    warmup();
    for &size in &[8 * KB, 32 * KB, 128 * KB] {
        let r = measure("entity_heavy", "xpath", || entity_heavy(size), &["xpath://zzz"]);
        assert!(
            r.retained_ratio() <= 5.0,
            "entity_heavy/xpath retained {:.1}x at {} — the quadratic blowup has regressed",
            r.retained_ratio(),
            human(size),
        );
    }
}
