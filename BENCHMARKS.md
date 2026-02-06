# Benchmarks

Measured on a ~50KB HTML document (200 vehicle listings) using [Criterion.rs](https://github.com/bheisler/criterion.rs).

Run them yourself with:

```sh
cargo bench
```

## First Match (single extraction)

| Engine      | Time     | Relative |
|-------------|----------|----------|
| **CSS**     | ~110 µs  | 🏆 1×    |
| **JMESPath**| ~280 µs  | 2.5×     |
| **Regex**   | ~527 µs  | 4.8×     |
| **XPath**   | ~2.96 ms | 27×      |

## All 200 Matches

| Engine      | Time     |
|-------------|----------|
| **CSS**     | ~113 µs  |
| **JMESPath**| ~309 µs  |
| **Regex**   | ~521 µs  |
| **XPath**   | ~2.91 ms |

## Post-Processing Overhead (`>>` functions)

| Query                                  | Time     |
|----------------------------------------|----------|
| `css:.vin`                             | ~110 µs  |
| `css:.vin >> normalize-space()`        | ~123 µs  |
| `css:.vin >> substring-after() >> uppercase()` | ~122 µs  |
| `xpath://… >> substring-after() >> uppercase()` | ~2.78 ms |
| `regex:VIN:\s*([\w]+)`                 | ~664 µs  |

## `select_first` Fallback Chain

| Scenario                        | Time     |
|---------------------------------|----------|
| Hit on 1st query                | ~97 µs   |
| Miss → Miss → Hit (3 queries)  | ~1.26 ms |

## CSS Index Scaling

| Index | Time     |
|-------|----------|
| 0     | ~100 µs  |
| 49    | ~100 µs  |
| 99    | ~101 µs  |
| 199   | ~100 µs  |

## Takeaways

- **CSS is the fastest engine** for HTML — the DOM is parsed and cached on first query; subsequent calls reuse it.
- **Post-processing is nearly free** — piping through `>>` functions adds <15 µs of overhead.
- **CSS index scaling is flat** — grabbing the 1st or 200th match costs the same, thanks to lazy caching.
- **XPath is ~27× slower** but offers the most expressive query language (union, axes, built-in functions).
- **Regex is the middle ground** — no DOM overhead, but scanning raw text at scale adds up.
- **`select_first` short-circuits** — if the first query hits, you pay only for that one engine.
