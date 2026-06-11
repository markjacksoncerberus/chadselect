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

## Per-query allocation churn (distinct from residency — fixed in 0.4.3)

The harness above measures **retained bytes** with **non-matching** selectors, so
it deliberately isolates parse/DOM cost and never sees per-query churn. A
separate failure mode surfaced from a consumer's per-extraction allocation guard:
one `extract_vehicle_data` over a 580 KiB VDP made **~425k short-lived
allocations / ~61 MiB**. Attribution (`tests/alloc_attribution.rs`) showed:

- The DOM build is **~1.6%** of that — parsing is cheap. The churn is per-query.
- **XPath is 100–1000× more alloc-hungry per result than CSS/regex.** A
  `//a[…]//b[…]/text()` shape allocated **~141 times per matched row**; CSS and
  regex are ~5–8/result. ~90% of a real extraction's allocations were XPath.

Root cause was structural churn in the chadpath evaluator (not the adapter):
`compose` rebuilt the whole evaluation `Context` with `ContextBuilder::from`
(a full deep-clone) **every path step**, and because predicates are themselves
`compose`s evaluated **per context node**, this multiplied to tens of thousands
of full-Context clones for a single query over a few-hundred-node page
(33,819 clones / 200 rows in the probe). `ContextBuilder::from` also cloned the
context sequence **twice**.

### The fix (chadpath 0.3.2 + adapter), ~45% fewer allocs/row on that shape

Measured on `//li[contains(@class,'spec-item')]//span[@class='value']/text()`,
allocs **per matched row** (slope cancels fixed overhead):

| change | allocs/row |
|---|---|
| baseline (0.4.2 / chadpath 0.3.1) | 140.8 |
| `compose` evolves one Context in place (no per-step clone) | 108.7 |
| `filter_seq` reuses the per-item context Vec | 105.7 |
| predicate paths use `ctxt.clone()` not `from` (drop 2nd seq clone) | 93.6 |
| cached `Rc<Value>` for boolean comparison results | 91.6 |
| **adapter:** concrete (un-boxed) axis iterators (`ENodeIter`) | **77.6** |

The adapter change is in `src/engine/xnode.rs`: `Node::NodeIterator` is an
associated type (not forced to `Box<dyn>`), so `child_iter`/`descend_iter`/
`attribute_iter`/… now return a concrete `ENodeIter` enum and no longer
heap-allocate a `Box` per axis call. All chadpath changes are behaviour-
preserving (full chadselect + chadpath test suites pass).

`tests/alloc_attribution.rs` is the permanent guard: it asserts XPath stays
under **95 allocs/matched row** (slope of 100→400 rows) and prints a per-engine
attribution table in `report` mode.

### Still open (CPU, not allocations)
`ENode::name()` clones a qualname `QName` per node on every name test, and
qualname's `UniqueString` takes a **global `RwLock` write lock on both clone and
drop** — uncontended single-threaded, but a serialization point under the
multi-threaded fleet. It does not heap-allocate (so it is invisible to the alloc
guard) but is a likely contributor to the "pegs the kernel" CPU symptom. A clean
fix would add a borrowed-name test method to the `Node` trait so name tests never
materialize a `QName`. Not yet done.

## Notes for very large pages

The XPath adapter rebuilds the document-order rank map once per `evaluate` call
(O(n)). If a single huge document is queried many times with `xpath:`, caching
that map alongside the parsed DOM on the `ContentItem` would remove the repeat
cost. Not currently a bottleneck.

## XPath conformance workarounds (0.3.1)

`xrust` has several XPath-1.0 conformance bugs in predicate evaluation. They are
worked around in `src/engine/xpath_rewrite.rs` by rewriting positional
predicates before evaluation:

| xrust bug | symptom | workaround |
|---|---|---|
| numeric predicate coerced with `to_bool()` | `[1]`, `[2]` keep **all** nodes | `name[N]` → `name[count(preceding-sibling::name)=N-1]` |
| `last()` in a predicate returns 1 (single-item context) | `[last()]` wrong | `name[last()]` → `name[not(following-sibling::name)]` |
| predicate applied over the **flattened** multi-parent node-set | `//tr/td[1]` returns only the first row's cell | per-parent sibling counting (same rewrites — they are relative to each node's own parent) |

Sibling counting is per-parent-correct and uses only axes + `count()` that xrust
evaluates correctly. Verified by `tests/xpath_conformance_probe.rs` (22 idioms)
and `tests/bug_a_positional.rs`.

**Residual limitations** (rare; not silently wrong in the common case):
- A positional predicate on a step with **no simple node test** — a chained
  predicate like `a[@x][2]` or a parenthesised `(//a)[2]` — falls back to
  `position()=N`, which is correct for a single-parent context but not across
  multiple parents (xrust's flattened-position bug remains there).
- `last()`-arithmetic beyond `last()-K` (e.g. `[last()-1>2]`) is left to xrust.

These stem from xrust's XPath engine, not chadselect. A complete fix would
require patching xrust's `filter()`/`compose()` (numeric→position conversion,
full-context `last()`, per-parent predicate grouping) — i.e. maintaining a
patched fork — or moving the XPath engine to a reference implementation
(libxml2) while keeping CSS on `scraper`.

## Parser stack-overflow guard (0.3.1)

xrust's recursive parser-combinators use ~130 KiB of stack per nesting level,
overflowing a 2 MiB tokio-worker stack at ~15 levels (`src/bin` repro from the
consumer). `engine/xpath.rs` routes expressions by `nesting_depth`: ≤8 inline,
9..2000 on a 512 MiB-stack thread (re-parsing, since `ENode` is `!Send`),
>2000 refused (empty + warning) — upholding the never-panics contract.
Verified by `tests/bug_b_deep_xpath.rs`.
