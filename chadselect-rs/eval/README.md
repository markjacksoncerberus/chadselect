# Real-selector performance suite

Replays the **actual** CSS/XPath selectors the `hermes_rust` crawler fleet feeds
chadselect against one giant synthetic dealer page, and ranks them by warm
per-evaluation CPU. Built to find the source of the post-0.3.x production CPU
regression (memory fixed, CPU still pegs at ~100%).

## Files

| File | What |
|------|------|
| `extract_selectors.py` | Lexes Rust string literals out of `hermes_rust/src/crawlers/*.rs` (handles `\`-newline continuations + raw strings + `>>` pipe stripping). Emits `selectors.json`. |
| `selectors.json` | 690 unique `css:` + 1396 unique `xpath:` selectors (generated). |
| `corpus_vocab.json` | tag / class-token / data-attr / id / label-text vocabulary mined from the selectors, so the synthetic HTML matches real nodes (generated). |
| `results.csv` | Full warm-timing ranking from the last run (generated). |
| `../examples/selector_eval.rs` | The suite: builds the corpus, warm-times every selector, ranks the slowest, aggregates cost by structural feature, and runs a 1×/2×/4× scaling probe to separate O(n) from O(n²). |
| `../examples/xpath_floor_probe.rs` | Minimal probe proving the per-query XPath floor (trivial `//title` vs equivalent `css:title`). |

## Regenerate / run

```bash
# 1. re-extract selectors + vocab from the live crawler sources
python3 eval/extract_selectors.py        # → eval/selectors.json
                                          #   (eval/corpus_vocab.json is mined the same way)

# 2. run the suite (release is essential — debug XPath eval is ~10× slower)
cargo run --release --example selector_eval -- --blocks 150 --top 30 --csv eval/results.csv

# focused variants
cargo run --release --example selector_eval -- --engine xpath --blocks 300
cargo run --release --example xpath_floor_probe
```

Flags: `--blocks N` (vehicle cards in the corpus), `--top N` (rows to print),
`--engine css|xpath`, `--budget-ms N` (per-selector warm budget), `--csv PATH`.

## Headline finding (150-card / ~1.95 MB page)

* Replaying one page's worth of fleet selectors = **103 s of CPU** for the 2086
  selectors; XPath alone is **88 s**, CSS **0.67 s** (XPath ~132× CSS in aggregate).
* There is **no cheap XPath**: the floor is `//title/text()[1]` at **3.78 ms**
  (matches nothing); median XPath **54 ms**, slowest **230 ms** — vs CSS median
  **0.78 ms**.
* **Scaling exponent ≈ 1.0 across the board** → the old O(n²) predicate bug is
  genuinely fixed; this is *linear cost with a huge constant*.
* 1282 / 1396 XPath selectors match **0 nodes** yet still average **61 ms** each —
  pure whole-document sweep cost.

Root cause: **97 % of fleet XPaths are `//`-rooted**, so every query
materializes a whole-document node set, and every query rebuilds the
document-order map (`ENode::root_of` → `build_order`) that the parsed DOM
deliberately caches but the order map does not. See the top-level analysis /
`xpath-cpu-regression` memory.

## Optimisation results (chadselect 0.4.1 / chadpath 0.3.1)

Driven against this suite as the marker. Four changes (all behaviour-preserving,
full XPath conformance suite still green, scaling still linear at exp ≈ 1.0):

1. **Order-map cache** — `ContentItem.html_order` caches the document-order map
   so it's built once per parsed document, not once per query
   (`evaluate_with_order` / `ENode::root_with_order`).
2. **`*`-wildcard `NameTest` fast-path** (chadpath) — a bare `*` step matches on
   node type without building (then discarding) a `QName` per node. `//*` is the
   hottest fleet shape.
3. **No-alloc `ENode::name()`** — the borrowed element name goes straight to the
   QName memo instead of a per-node `String` allocation.
4. **Lazy axis iterators** — `child`/`descendant`/sibling/`ancestor` iterate node
   ids without materialising a `Vec<ENode>` per call (`descendant` no longer
   buffers all N nodes for `//`).

| Metric (blocks=150) | 0.3.0 baseline | 0.4.1 / 0.3.1 | Δ |
|---|---|---|---|
| Suite (2086 selectors) | 103.4 s | **84.5 s** | **−18 %** |
| `rooted //` mean | 63.3 ms | 49.5 ms | −22 % |
| `contains()` mean | 69.9 ms | 55.5 ms | −21 % |
| `sibling axis` mean | 72.6 ms | 57.5 ms | −21 % |
| worst selector | 230.6 ms | 208.4 ms | −10 % |
| `//title` floor (16k nodes) | 12.3 ms | 7.2 ms | −41 % |
| `/html/body` floor | 2.05 ms | 0.28 ms | 7.3× |

Tried and **reverted**: skipping chadpath's per-step sort/dedup for
single-context forward axes — no measurable gain, not worth a behaviour change to
the generic engine. The remaining heavy cost is per-node predicate *dispatch*
over the full `//*` set; the biggest real-world lever now is anchoring
`//*[…]` → `//tag[…]` in the crawler selectors themselves.
