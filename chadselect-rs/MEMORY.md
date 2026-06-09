# HTML Parsing Memory Behavior

This crate parses HTML with `scraper`/html5ever into a single `ego-tree` DOM
that is **shared** by both the CSS engine (native `scraper` selectors) and the
XPath engine (`xrust` evaluating over the same tree via the
[`ENode`](src/engine/xnode.rs) adapter). A document is parsed **once**,
regardless of how many or which kinds of queries run against it.

`tests/memory_harness.rs` measures parse-time memory amplification with a
tracking global allocator declared **only** in the test crate — the published
library and its consumers are unaffected and gain no dependency.

## Running

```sh
# Diagnostic report (ratio table; never asserts), ~9s:
cargo test --release --test memory_harness report \
    -- --include-ignored --nocapture --test-threads=1

# Regression / safety guards:
cargo test --release --test memory_harness guard \
    -- --include-ignored --test-threads=1
```

`--test-threads=1` is required: the allocator counter is process-global.
Metrics (ratio = `bytes / input_len`): **peak×** transient heap during parse +
eval; **ret×** heap retained while the document is alive; **leak** heap still
outstanding after `Drop`.

## History — the XPath memory explosion (fixed)

The original XPath engine used `sxd_html` → `sxd-document`, whose tree-build was
**quadratic in memory and time** on real-world and adversarial HTML:

| pattern | input | OLD xpath retained | OLD ratio |
|---|---|---|---|
| entity-dense text | 8 KB | 1.8 MB | 227× |
| entity-dense text | 32 KB | 36 MB | 1152× |
| entity-dense text | 128 KB | 421 MB | 3367× |
| entity-dense text | 256 KB | **1.6 GB** | **6547×** |

Entities (`&amp;`, `&nbsp;`, `&#65;`) are ubiquitous, so a multi-MB page would
extrapolate to **tens of GB** — the production OOM. The path was also O(n²) in
*time* (a 1 MB `tiny_repeated` doc took 27 s on xpath vs 81 ms on css).

### The fix

The XPath engine was moved off `sxd_html` onto the **same html5ever DOM the CSS
engine already builds**, evaluated by `xrust` through a read-only `Node`-trait
adapter (`src/engine/xnode.rs`). This:

- eliminates the second parse and the separate `sxd-document` tree;
- removes the quadratic memory blow-up entirely;
- removes the dual-DOM doubling (querying one doc with both `css:` and `xpath:`
  now holds a single shared tree).

Two adapter details are load-bearing:
- **Document-order ranks** are precomputed once per document (a pre-order index
  map), so `cmp_document_order` is O(1); otherwise xrust's per-step nodeset sort
  is O(n²).
- **QName construction is memoized** (`QNAME_MEMO`). qualname's interner takes a
  global write-lock and linear-scans on every `NcName::try_from` (~58 µs/call);
  `name()` runs once per visited node, so without memoization a single `//x`
  over a 20 k-element page cost ~15 s. Memoizing collapses it to O(distinct names).

### After the fix (retained ratios now equal the css path)

| pattern | input | css ret× | xpath ret× | xpath time |
|---|---|---|---|---|
| entity-dense text | 128 KB | 1.3× | **1.3×** | 16 ms |
| well_formed | 256 KB | 19× | 19× | 18 ms |
| tiny_repeated | 256 KB | 47× | 47× | 45 ms |
| misnested | 128 KB | 93× | 93× | 49 ms |
| unclosed | 128 KB | 47× | 47× | 2.6 s* |

\* `unclosed`/`misnested` time is html5ever's own adoption-agency cost and is
identical on the css path — it is a property of parsing that shape of broken
HTML, not of the XPath engine.

## Current characteristics

- **No memory leaks.** Every pattern returns to baseline after `Drop`
  (`guard_no_leaks_across_patterns`). A one-time ~56 KB on the first measurement
  is allocator/interner warmup.
- **Retained amplification is bounded and linear** for both engines
  (`guard_retained_is_bounded`, ≤ 120×). The DOM is inherently larger than the
  source bytes (per-node overhead); this is steady and size-independent, not a
  leak or a blow-up.
- **`guard_xpath_entity_no_longer_explodes`** locks in the fix: entity-dense
  xpath retained stays ≤ 5× and does not grow with input size.
- **Transient peak** for xpath includes xrust's evaluation working-set (a bounded
  ~2–3 MB plus an O(n) node-handle vector); it does not persist and does not grow
  super-linearly.
- **`element_text_cache`** (CSS text pseudo-selectors) plateaus (~4.4 MB on a
  256 KB doc) and is not a leak.

## Notes for very large pages

The XPath adapter rebuilds the document-order rank map once per `evaluate` call
(O(n)). If a single huge document is queried many times with `xpath:`, caching
that map alongside the parsed DOM on the `ContentItem` would remove the repeat
cost. Not currently a bottleneck.
