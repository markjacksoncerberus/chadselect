# ChadSelect Benchmarks

**Rust vs Python** — same fixtures (200 items), same 20 queries, same machine.

> Rust: `cargo bench` (Criterion, 100 samples)
> Python: `python tests/bench.py -n 2000`

---

## CSS Selectors

Rust uses `scraper` (pure Rust). Python uses `selectolax` (C/lexbor).

| Task | Rust | Python | Winner |
|------|-----:|-------:|--------|
| product titles (×200) | **89 µs** | 1,800 µs | 🦀 Rust 20× |
| current prices (×200) | **95 µs** | 1,800 µs | 🦀 Rust 19× |
| buy link hrefs (×200) | **76 µs** | 1,800 µs | 🦀 Rust 24× |
| ticker symbols (×20) | **3.5 µs** | 41 µs | 🦀 Rust 12× |
| first VIN + chain (×1) | **104 µs** | 1,800 µs | 🦀 Rust 17× |
| all data-sku attrs (×200) | **96 µs** | 1,700 µs | 🦀 Rust 18× |

```
product titles     🦀 ████                          89 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  1,800 µs

current prices     🦀 █████                         95 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  1,800 µs

ticker symbols     🦀 ██                            3.5 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  41 µs
```

---

## XPath

Rust uses `sxd-xpath` (pure Rust). Python uses `lxml` (C/libxml2).

| Task | Rust | Python | Winner |
|------|-----:|-------:|--------|
| current prices (×200) | **2,413 µs** | 2,700 µs | 🦀 Rust 1.1× |
| buy link hrefs (×200) | **1,875 µs** | 2,500 µs | 🦀 Rust 1.3× |
| headline (×1) | **14 µs** | 45 µs | 🦀 Rust 3.3× |
| all data-skus (×200) | **2,216 µs** | 2,600 µs | 🦀 Rust 1.2× |
| union h1\|h2 (×200) | 2,645 µs | **2,400 µs** | 🐍 Python 1.1× |
| normalize-space (×1) | **11 µs** | 44 µs | 🦀 Rust 3.9× |

```
current prices     🦀 ████████████████████████████████████████████████████████████████████████          2,413 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  2,700 µs

headline           🦀 █████████████████████████      14 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  45 µs

normalize-space    🦀 ████████████████████           11 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  44 µs
```

---

## Regex

Rust uses `regex` (pure Rust). Python uses `re` (C-backed stdlib).

| Task | Rust | Python | Winner |
|------|-----:|-------:|--------|
| dollar prices (×400) | 192 µs | **106 µs** | 🐍 Python 1.8× |
| VIN numbers (×200) | 470 µs | **107 µs** | 🐍 Python 4.4× |
| data-sku capture (×200) | 155 µs | **98 µs** | 🐍 Python 1.6× |
| pct changes (×20) | 110 µs | **10 µs** | 🐍 Python 11× |

```
dollar prices      🦀 ██████████████████████████████████████████████████████████████████████████████████  192 µs
                   🐍 ████████████████████████████████████████████                                       106 µs

VIN numbers        🦀 ██████████████████████████████████████████████████████████████████████████████████  470 µs
                   🐍 ██████████████████                                                                 107 µs

pct changes        🦀 ██████████████████████████████████████████████████████████████████████████████████  110 µs
                   🐍 ███████                                                                            10 µs
```

---

## JMESPath

Both use pure-language `jmespath` crates/packages.

| Task | Rust | Python | Winner |
|------|-----:|-------:|--------|
| product names (×200) | 380 µs | **374 µs** | ≈ tie |
| in-stock filter (×133) | **382 µs** | 426 µs | 🦀 Rust 1.1× |
| nested spec (×1) | 351 µs | **235 µs** | 🐍 Python 1.5× |
| flatten categories (×400) | **395 µs** | 567 µs | 🦀 Rust 1.4× |

```
product names      🦀 █████████████████████████████████████████████████████████████████████████████████  380 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  374 µs

flatten categories 🦀 ████████████████████████████████████████████████████████                          395 µs
                   🐍 ██████████████████████████████████████████████████████████████████████████████████  567 µs
```

---

## Summary

| Engine | Rust | Python | Notes |
|--------|------|--------|-------|
| **CSS** | 🦀 **12–24× faster** | — | Rust `scraper` dominates selectolax |
| **XPath** | 🦀 **1.1–3.9× faster** | Close on bulk | Both backed by mature parsers |
| **Regex** | — | 🐍 **1.6–11× faster** | Python `re` is C-backed; Rust re-parses HTML each call |
| **JMESPath** | Mixed | Mixed | Roughly equivalent; both pure-language impls |

**Bottom line:** Rust wins big on CSS and XPath. Python wins on regex (C-backed stdlib advantage). JMESPath is a wash. Pick the language that fits your stack — the API is identical either way.

---

## Real-selector performance suite (`chadselect-rs/eval/`)

The micro-benches above use a handful of hand-written queries. The **real**
production load is ~2,000 selectors from the `hermes_rust` crawler fleet — and a
post-0.3.x CPU regression only showed up there. The eval suite replays every
real `css:`/`xpath:` selector against one ~2 MB synthetic dealer page and ranks
them by warm per-evaluation CPU (see `eval/README.md`).

What it found: the old O(n²) predicate bug was genuinely fixed (scaling exponent
~1.0), but **every XPath query paid a large per-query whole-document cost** —
97 % of fleet XPaths are `//`-rooted, and the document-order map was rebuilt on
every query. XPath was ~130× the CSS cost in aggregate.

`chadselect 0.4.1` / `chadpath 0.3.1` attack that (order-map cache, `*`-wildcard
fast-path, no-alloc `name()`, lazy axis iterators):

| Metric (blocks=150, 2086 selectors) | 0.3.0 | 0.4.1 | Δ |
|---|---:|---:|---|
| Full warm-timing pass | 103.4 s | **84.5 s** | −18 % |
| `//`-rooted XPath (mean) | 63.3 ms | 49.5 ms | −22 % |
| `contains()` XPath (mean) | 69.9 ms | 55.5 ms | −21 % |
| trivial `//title` (16k-node page) | 12.3 ms | 7.2 ms | −41 % |
| `/html/body` (per-query overhead) | 2.05 ms | 0.28 ms | 7.3× |

> Run: `cargo run --release --example selector_eval -- --blocks 150 --csv eval/results.csv`
