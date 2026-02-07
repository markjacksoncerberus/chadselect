# ChadSelect

**One query. Any format. Every selector.**

ChadSelect unifies **Regex**, **XPath 1.0**, **CSS Selectors**, and **JMESPath** behind a single, dead-simple query interface. Load your content, prefix your query string, get results. That's it.

Available as a **Rust crate** and a **Python package** — same API, same query syntax, same behavior.

| Package | Install | Engines |
|---------|---------|---------|
| [**chadselect** (Rust)](chadselect-rs/) | `cargo add chadselect` | regex, sxd-xpath, scraper, jmespath |
| [**chadselect** (Python)](chadselect-py/) | `pip install chadselect` | re, lxml, selectolax, jmespath |

---

## Why ChadSelect?

Scraping and data extraction is messy. You're juggling different libraries for regex, CSS, XPath, and JSON — all with different APIs, different error handling, and different mental models.

ChadSelect collapses all of that into one class and one query pattern:

```python
from chadselect import ChadSelect

cs = ChadSelect()
cs.add_html(html)
cs.add_json(json_str)

cs.select(0, "css:.price")                          # CSS selector
cs.select(0, "xpath://span[@id='vin']/text()")       # XPath
cs.select(0, r"regex:VIN:\s*(\w+)")                  # Regex
cs.select(0, "json:inventory[0].price")              # JMESPath
```

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();
cs.add_html(html);
cs.add_json(json);

cs.select(0, "css:.price");
cs.select(0, "xpath://span[@id='vin']/text()");
cs.select(0, r"regex:VIN:\s*(\w+)");
cs.select(0, "json:inventory[0].price");
```

No separate parsers. No error handling boilerplate. Queries that fail return empty strings — **never panics/raises**.

---

## Features

- **Unified query API** — `regex:`, `xpath:`, `css:`, `json:` prefixes route to the right engine
- **Multi-content** — load multiple documents (HTML, JSON, plain text) and query across all of them
- **Post-processing functions** — pipe results through `normalize-space()`, `uppercase()`, `substring-after()`, and more via `>>`
- **CSS text pseudo-selectors** — `:has-text()`, `:text-equals()`, `:text-starts()`, `:text-ends()`, `:contains-text()`
- **Index selection** — grab all results, the first, the Nth, or fallback through a priority list
- **Custom validators** — `select_where`, `select_first_where`, `select_many_where` accept a callback to filter results
- **Zero panics** — every code path returns empty results on failure, never crashes

---

## Quick Start

### Python

```bash
pip install chadselect
```

```python
from chadselect import ChadSelect

cs = ChadSelect()
cs.add_html('<div class="product"><span class="price">$49.99</span></div>')

price = cs.select(0, "css:.price")
# => "$49.99"
```

### Rust

```toml
[dependencies]
chadselect = "0.2.0"
```

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();
cs.add_html(r#"<div class="product"><span class="price">$49.99</span></div>"#.to_string());

let price = cs.select(0, "css:.price");
assert_eq!(price, "$49.99");
```

---

## Query Prefixes

| Prefix    | Engine   | Content Types | Example |
|-----------|----------|---------------|---------|
| `regex:`  | Regex    | All           | `regex:price:\s*\$(\d+)` |
| `xpath:`  | XPath 1.0| HTML, Text    | `xpath://span[@class='vin']/text()` |
| `css:`    | CSS      | HTML          | `css:div.product > .price` |
| `json:`   | JMESPath | JSON          | `json:store.inventory[0].name` |

No prefix defaults to **regex**.

---

## API Overview

Both languages expose the same methods:

| Method | Description |
|--------|-------------|
| `select(index, query)` | Single result (string). `-1` = search all docs, `N` = Nth match. Empty on no match. |
| `query(index, query)` | All results (list/vec). `-1` = all matches across all docs, `N` = Nth match. |
| `select_first(queries)` | Try queries in order, return first hit. |
| `select_many(queries)` | Combine unique results from multiple queries. |
| `select_where(index, query, validator)` | Like `select`, but filter with a callback. |
| `select_first_where(queries, validator)` | Like `select_first`, with validation. |
| `select_many_where(queries, validator)` | Like `select_many`, with validation. |
| `query_batch(queries)` | Run many queries, return list of result lists. |

### Content Management

```python
cs.add_html(html)   # HTML (CSS, XPath, Regex)
cs.add_json(json)   # JSON (JMESPath, Regex)
cs.add_text(text)   # Plain text (Regex, XPath)
cs.content_count()   # Number of loaded documents
cs.clear()           # Remove all content
```

### Fallback Chains

```python
result = cs.select_first([
    (0, "css:#exact-id"),
    (0, "xpath://span[@class='alt']/text()"),
    (0, r"regex:fallback:\s*(.+)"),
])
```

### Custom Validators

```python
# Reject "0" as a price
price = cs.select_where(0, "css:.price", lambda s: s != "0")

# Minimum length
vin = cs.select_where(0, "css:.vin", lambda s: len(s) >= 17)

# Numeric range
price = cs.select_where(0, "json:price", lambda s: float(s) > 10.0)
```

---

## Post-Processing Functions

Pipe results through text functions using `>>` (not `|`, which is reserved for XPath union and JMESPath pipe).

```
css:.selector >> function1() >> function2()
xpath://path/text() >> function1() >> function2()
```

| Function | Description | Example |
|----------|-------------|---------|
| `normalize-space()` | Trim + collapse whitespace | `css:.desc >> normalize-space()` |
| `trim()` | Trim edges only | `css:.title >> trim()` |
| `uppercase()` | UPPER CASE | `css:.vin >> uppercase()` |
| `lowercase()` | lower case | `css:.name >> lowercase()` |
| `substring(start, len)` | Extract substring | `css:.code >> substring(0, 3)` |
| `substring-after('delim')` | Text after delimiter | `css:.vin >> substring-after('VIN: ')` |
| `substring-before('delim')` | Text before delimiter | `css:.info >> substring-before(': ')` |
| `replace('find', 'repl')` | Replace text | `css:.price >> replace('$', 'USD ')` |
| `get-attr('name')` | Element attribute (CSS) | `css:a >> get-attr('href')` |

### Chaining

```python
result = cs.select(0, "css:.vin >> substring-after('VIN: ') >> substring(0, 3) >> lowercase()")
# => "1hg"
```

---

## CSS Text Pseudo-Selectors

| Pseudo-Selector | Behavior |
|-----------------|----------|
| `:has-text('x')` | Element or descendants contain the text |
| `:contains-text('x')` | Element's own text contains the text |
| `:text-equals('x')` | Text exactly equals |
| `:text-starts('x')` | Text starts with |
| `:text-ends('x')` | Text ends with |

```python
color = cs.select(0, "css:.item:has-text('Exterior:') .value")
# => "Blue Metallic"

result = cs.select(0, "css:.item:has-text('Interior:') .value >> uppercase()")
# => "BLACK LEATHER"
```

---

## Repository Structure

```
chadselect/
├── README.md                     # This file
├── chadselect-rs/                # Rust crate (crates.io)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── content.rs
│   │   ├── query.rs
│   │   ├── functions.rs
│   │   └── engine/ (css, xpath, regex, json)
│   └── tests/
├── chadselect-py/                # Python package (PyPI)
│   ├── pyproject.toml
│   ├── src/chadselect/
│   │   ├── __init__.py
│   │   ├── _chadselect.py
│   │   ├── _query.py
│   │   ├── _functions.py
│   │   └── engine/ (css, xpath, regex, json)
│   └── tests/
└── .github/workflows/
    ├── publish-cargo.yml          # crates.io publish
    └── publish-pypi.yml          # PyPI publish
```

---

## Design Principles

1. **Never panic** — invalid queries, malformed content, out-of-bounds indices all return empty results
2. **Prefix routing** — the query string declares the engine; no mode switching or builder patterns
3. **`>>` function pipe** — unambiguous across all engines; XPath `|` union works natively
4. **Batteries included** — post-processing, text pseudo-selectors, validators, and index selection are all built in

---

## License

MIT
