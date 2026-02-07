# ChadSelect

**One query. Any format. Every selector.**

Unified data extraction — **Regex**, **XPath 1.0**, **CSS Selectors**, and **JMESPath** behind one query interface. Load your content, prefix your query, get results. Never panics.

[![Crates.io](https://img.shields.io/crates/v/chadselect.svg)](https://crates.io/crates/chadselect)
[![docs.rs](https://docs.rs/chadselect/badge.svg)](https://docs.rs/chadselect)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();
cs.add_html(r#"<span class="price">$49.99</span>"#.to_string());

let price = cs.select(0, "css:.price");
assert_eq!(price, "$49.99");
```

## Install

```toml
[dependencies]
chadselect = "0.2.0"
```

---

## Query Syntax

Every query uses an `engine:expression` prefix. No prefix defaults to regex.

| Prefix | Engine | Content Types | Backed By |
|--------|--------|---------------|-----------|
| `css:` | CSS Selectors | HTML | [scraper](https://crates.io/crates/scraper) |
| `xpath:` | XPath 1.0 | HTML, Text | [sxd-xpath](https://crates.io/crates/sxd-xpath) |
| `regex:` | Regular Expressions | All | [regex](https://crates.io/crates/regex) |
| `json:` | JMESPath | JSON | [jmespath](https://crates.io/crates/jmespath) |

---

## The `index` Parameter

Every query method takes an `index` argument that controls **which match** to return:

| Value | Behavior |
|-------|----------|
| `-1` | Return **all** matches across every loaded document |
| `0` | Return only the **first** match |
| `N` | Return only the **Nth** match (0-based) |

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<ul><li>A</li><li>B</li><li>C</li></ul>"#.to_string());

let all   = cs.query(-1, "css:li");  // vec!["A", "B", "C"]
let first = cs.query(0,  "css:li");  // vec!["A"]
let third = cs.query(2,  "css:li");  // vec!["C"]
let oob   = cs.query(99, "css:li");  // vec![]  (out of bounds — never panics)

// select() wraps query() — returns a single String
let s = cs.select(0, "css:li");       // "A"
let s = cs.select(-1, "css:li");      // "A" (first of all matches)
```

When multiple documents are loaded, `-1` aggregates results from **all** compatible documents before indexing.

---

## Content Management

Load one or more documents. Each document is tagged by type and only queried by compatible engines.

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();

// HTML — compatible with css:, xpath:, regex:
cs.add_html(r#"
<html>
  <body>
    <h1 class="title">2024 Honda Civic</h1>
    <span class="price">$28,500</span>
    <div class="details">
      <div class="item"><span class="label">VIN:</span> 1HGFE2F59PA000001</div>
      <div class="item"><span class="label">Exterior:</span> Blue Metallic</div>
      <div class="item"><span class="label">Interior:</span> Black Leather</div>
      <div class="item"><span class="label">Mileage:</span> 12,345 mi</div>
    </div>
    <a class="dealer-link" href="https://example.com/dealer/42">View Dealer</a>
  </body>
</html>
"#.to_string());

// JSON — compatible with json:, regex:
cs.add_json(r#"{
  "inventory": [
    {"id": 1, "name": "Civic",   "price": 28500, "tags": ["sedan", "honda"]},
    {"id": 2, "name": "Accord",  "price": 34000, "tags": ["sedan", "honda"]},
    {"id": 3, "name": "CR-V",    "price": 32500, "tags": ["suv",   "honda"]}
  ],
  "dealer": {"name": "Metro Honda", "rating": 4.8}
}"#.to_string());

// Plain text — compatible with regex:, xpath:
cs.add_text("Order #12345 confirmed. Total: $99.50".to_string());

assert_eq!(cs.content_count(), 3);

cs.clear(); // remove all content
```

---

## CSS Selectors

Standard CSS selectors, plus custom text pseudo-selectors for scraping.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"
<ul class="products">
  <li class="product" data-id="1"><span class="name">Widget</span><span class="price">$19.99</span></li>
  <li class="product" data-id="2"><span class="name">Gadget</span><span class="price">$49.99</span></li>
  <li class="product" data-id="3"><span class="name">Doohickey</span><span class="price">$9.99</span></li>
</ul>
"#.to_string());

// Basic selectors
let first_name = cs.select(0, "css:.product .name");
assert_eq!(first_name, "Widget");

// All matches — index -1
let all_prices = cs.query(-1, "css:.product .price");
assert_eq!(all_prices, vec!["$19.99", "$49.99", "$9.99"]);

// Nth match — index 2 (0-based)
let third = cs.query(2, "css:.product .name");
assert_eq!(third, vec!["Doohickey"]);

// Attribute extraction via get-attr()
let id = cs.select(0, "css:.product >> get-attr('data-id')");
assert_eq!(id, "1");
```

### Text Pseudo-Selectors

These work like Playwright's pseudo-selectors — match elements by text content.

| Pseudo-Selector | Behavior |
|-----------------|----------|
| `:has-text('x')` | Element **or its descendants** contain the text |
| `:contains-text('x')` | Element's **own** text contains the text |
| `:text-equals('x')` | Element's text **exactly** equals |
| `:text-starts('x')` | Element's text **starts** with |
| `:text-ends('x')` | Element's text **ends** with |

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"
<div class="specs">
  <div class="row"><span class="label">Exterior</span><span class="value">Blue Metallic</span></div>
  <div class="row"><span class="label">Interior</span><span class="value">Black Leather</span></div>
  <div class="row"><span class="label">Engine</span><span class="value">2.0L Turbo</span></div>
</div>
"#.to_string());

// :has-text — matches the .row whose subtree contains "Exterior"
let color = cs.select(0, "css:.row:has-text('Exterior') .value");
assert_eq!(color, "Blue Metallic");

// :text-equals — exact match on element text
let engine_label = cs.select(0, "css:.label:text-equals('Engine')");
assert_eq!(engine_label, "Engine");

// :text-starts — prefix match
let starts_e = cs.select(0, "css:.label:text-starts('Ext')");
assert_eq!(starts_e, "Exterior");

// :text-ends — suffix match
let ends_or = cs.select(0, "css:.label:text-ends('ior')");
assert_eq!(ends_or, "Exterior");

// Combine with function piping
let upper_interior = cs.select(0, "css:.row:has-text('Interior') .value >> uppercase()");
assert_eq!(upper_interior, "BLACK LEATHER");
```

---

## XPath 1.0

Full XPath 1.0 support including axes, predicates, and XPath functions.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"
<html>
  <body>
    <h1 id="title">  2024 Honda Civic  </h1>
    <table class="specs">
      <tr><td>VIN</td><td>1HGFE2F59PA000001</td></tr>
      <tr><td>Price</td><td>$28,500</td></tr>
      <tr><td>Mileage</td><td>12,345 mi</td></tr>
    </table>
  </body>
</html>
"#.to_string());

// text() extraction
let title = cs.select(0, "xpath://h1[@id='title']/text()");
assert_eq!(title, "  2024 Honda Civic  ");

// With normalize-space
let clean_title = cs.select(0, "xpath:normalize-space(//h1[@id='title'])");
assert_eq!(clean_title, "2024 Honda Civic");

// Predicate-based selection — find the <td> after "VIN"
let vin = cs.select(0, "xpath://tr[td='VIN']/td[2]/text()");
assert_eq!(vin, "1HGFE2F59PA000001");

// All values from the second column
let all_values = cs.query(-1, "xpath://table[@class='specs']//tr/td[2]/text()");
assert_eq!(all_values, vec!["1HGFE2F59PA000001", "$28,500", "12,345 mi"]);

// XPath string() on attribute
let title_id = cs.select(0, "xpath:string(//h1/@id)");
assert_eq!(title_id, "title");
```

---

## Regex

Capture groups or full matches. Works on HTML, JSON, and plain text content.

```rust
let mut cs = ChadSelect::new();
cs.add_text("VIN: 1HGFE2F59PA000001 | Stock #: A12345 | Price: $28,500".to_string());

// Capture group — returns the group, not the full match
let vin = cs.select(0, r"regex:VIN:\s*([A-HJ-NPR-Z0-9]{17})");
assert_eq!(vin, "1HGFE2F59PA000001");

// Full match — no capture group
let stock = cs.select(0, r"regex:Stock #:\s*\S+");
assert_eq!(stock, "Stock #: A12345");

// Multiple capture groups — returns first group
let price_digits = cs.select(0, r"regex:Price:\s*\$([0-9,]+)");
assert_eq!(price_digits, "28,500");

// All matches
let all_numbers = cs.query(-1, r"regex:\d+");
// Returns all digit sequences found in the text

// No prefix — defaults to regex
let vin2 = cs.select(0, r"[A-HJ-NPR-Z0-9]{17}");
assert_eq!(vin2, "1HGFE2F59PA000001");
```

### Regex on HTML

Regex runs on the raw HTML string, not parsed text — useful for extracting from attributes, comments, or script tags.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<script>var price = 28500;</script>"#.to_string());

let price = cs.select(0, r"regex:var price\s*=\s*(\d+)");
assert_eq!(price, "28500");
```

---

## JMESPath (JSON)

Full JMESPath expression support for structured JSON extraction.

```rust
let mut cs = ChadSelect::new();
cs.add_json(r#"{
  "inventory": [
    {"id": 1, "name": "Civic",   "price": 28500, "tags": ["sedan", "honda"]},
    {"id": 2, "name": "Accord",  "price": 34000, "tags": ["sedan", "honda"]},
    {"id": 3, "name": "CR-V",    "price": 32500, "tags": ["suv",   "honda"]}
  ],
  "dealer": {"name": "Metro Honda", "rating": 4.8}
}"#.to_string());

// Simple field access
let dealer = cs.select(0, "json:dealer.name");
assert_eq!(dealer, "Metro Honda");

// Array indexing
let first = cs.select(0, "json:inventory[0].name");
assert_eq!(first, "Civic");

// Projection — all names
let names = cs.query(-1, "json:inventory[*].name");
assert_eq!(names, vec!["Civic", "Accord", "CR-V"]);

// Filter expression
let expensive = cs.query(-1, "json:inventory[?price > `30000`].name");
assert_eq!(expensive, vec!["Accord", "CR-V"]);

// Nested access
let rating = cs.select(0, "json:dealer.rating");
assert_eq!(rating, "4.8");

// Flatten nested arrays
let all_tags = cs.query(-1, "json:inventory[*].tags[]");
assert_eq!(all_tags, vec!["sedan", "honda", "sedan", "honda", "suv", "honda"]);
```

---

## Post-Processing Functions

Pipe results through text transformations using `>>`. This operator was chosen over `|` because `|` is reserved by XPath (union) and JMESPath (pipe).

```
css:.selector >> function1() >> function2()
xpath://path/text() >> trim() >> uppercase()
regex:pattern >> replace('$', 'USD ')
```

| Function | Description | Example |
|----------|-------------|---------|
| `normalize-space()` | Trim + collapse internal whitespace | `css:.desc >> normalize-space()` |
| `trim()` | Trim leading/trailing whitespace | `css:.title >> trim()` |
| `uppercase()` | Convert to UPPER CASE | `css:.vin >> uppercase()` |
| `lowercase()` | Convert to lower case | `css:.name >> lowercase()` |
| `substring(start, len)` | Extract substring (0-based) | `css:.code >> substring(0, 3)` |
| `substring-after('delim')` | Text after first delimiter | `css:.info >> substring-after('VIN: ')` |
| `substring-before('delim')` | Text before first delimiter | `css:.info >> substring-before(': ')` |
| `replace('find', 'repl')` | Replace all occurrences | `css:.price >> replace('$', 'USD ')` |
| `get-attr('name')` | Element attribute (CSS only) | `css:a.link >> get-attr('href')` |

### Chaining Functions

Functions execute left-to-right. Empty results are filtered after each step.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<div class="info">  VIN: 1HGFE2F59PA000001  </div>"#.to_string());

// Chain: extract text → get everything after "VIN: " → first 3 chars → lowercase
let result = cs.select(0, "css:.info >> substring-after('VIN: ') >> substring(0, 3) >> lowercase()");
assert_eq!(result, "1hg");
```

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<a class="link" href="/inventory/123">View Car</a>"#.to_string());

// Attribute extraction
let href = cs.select(0, "css:a.link >> get-attr('href')");
assert_eq!(href, "/inventory/123");
```

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<span class="price">  $ 28,500  </span>"#.to_string());

// Clean + transform
let clean_price = cs.select(0, "css:.price >> normalize-space() >> replace('$ ', '$')");
assert_eq!(clean_price, "$28,500");
```

---

## API Reference

### Core Query Methods

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();
cs.add_html(html);

// query() — returns Vec<String>, never panics
let all_matches = cs.query(-1, "css:.price");   // all results
let first_only  = cs.query(0,  "css:.price");   // vec with 1st result or empty
let third       = cs.query(2,  "css:.price");   // vec with 3rd result or empty

// select() — returns String, empty on no match
let price = cs.select(0, "css:.price");          // first valid result or ""
```

### Fallback Chains — `select_first`

Try queries in priority order. Returns the first result set where all values pass validation.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<span class="alt-price">$28,500</span>"#.to_string());

// #exact-id doesn't exist, falls through to .alt-price
let result = cs.select_first(vec![
    (0, "css:#exact-id"),
    (0, "css:.alt-price"),
    (0, r"regex:\$[\d,]+"),
]);
assert_eq!(result, vec!["$28,500"]);
```

### Multi-Source — `select_many`

Combine unique results from multiple queries.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"
<span class="msrp">$30,000</span>
<span class="sale">$28,500</span>
"#.to_string());

let prices = cs.select_many(vec![
    (0, "css:.msrp"),
    (0, "css:.sale"),
]);
// Contains both "$30,000" and "$28,500" (unique, unordered)
assert!(prices.contains(&"$30,000".to_string()));
assert!(prices.contains(&"$28,500".to_string()));
```

### Custom Validators — `select_where`

Filter results with a closure. The `_where` variants exist for `select`, `select_first`, and `select_many`.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<span class="price">0</span><span class="price">28500</span>"#.to_string());

// Reject "0" as a valid price
let price = cs.select_where(0, "css:.price", |s| s != "0");
assert_eq!(price, ""); // first match "0" rejected, no fallback within select_where

// With select_first_where — falls through to next query
let mut cs2 = ChadSelect::new();
cs2.add_text("a: 0\nb: 42".to_string());

let r = cs2.select_first_where(
    vec![(0, r"a: (\d+)"), (0, r"b: (\d+)")],
    |s| s != "0",
);
assert_eq!(r, vec!["42"]);
```

### Batch Queries — `query_batch`

Execute many queries in one call. Returns `Vec<Vec<String>>` in input order.

```rust
let mut cs = ChadSelect::new();
cs.add_html(r#"<h1>Civic</h1><span class="price">$28,500</span>"#.to_string());
cs.add_json(r#"{"dealer": "Metro Honda"}"#.to_string());

let results = cs.query_batch(&[
    (0, "css:h1"),
    (0, "css:.price"),
    (0, "json:dealer"),
]);
assert_eq!(results[0], vec!["Civic"]);
assert_eq!(results[1], vec!["$28,500"]);
assert_eq!(results[2], vec!["Metro Honda"]);
```

---

## Multi-Content Queries

When multiple documents are loaded, queries search across all compatible content. Use `query(-1, ...)` to get results from every document.

```rust
let mut cs = ChadSelect::new();

cs.add_html(r#"<span class="title">Page 1</span>"#.to_string());
cs.add_html(r#"<span class="title">Page 2</span>"#.to_string());

// Searches both HTML documents
let titles = cs.query(-1, "css:.title");
assert_eq!(titles, vec!["Page 1", "Page 2"]);

// Mixing content types
cs.add_json(r#"{"title": "JSON Title"}"#.to_string());

// css: only queries HTML content — JSON is skipped
let html_titles = cs.query(-1, "css:.title");
assert_eq!(html_titles, vec!["Page 1", "Page 2"]);

// json: only queries JSON content
let json_title = cs.select(0, "json:title");
assert_eq!(json_title, "JSON Title");

// regex: searches everything
let all = cs.query(-1, r"regex:(?:Page \d|JSON Title)");
assert_eq!(all.len(), 3);
```

---

## Error Handling

ChadSelect **never panics**. Every invalid query, malformed content, or out-of-bounds index returns empty results.

```rust
let mut cs = ChadSelect::new();
cs.add_html("<div>hello</div>".to_string());

// Invalid CSS selector — returns ""
let r = cs.select(0, "css:][invalid");
assert_eq!(r, "");

// Out of bounds index — returns empty vec
let r = cs.query(999, "css:div");
assert_eq!(r, Vec::<String>::new());

// Wrong engine for content type — returns ""
cs.add_json(r#"{"a": 1}"#.to_string());
let r = cs.select(0, "css:.something"); // css: doesn't apply to JSON
// Only the HTML is searched, no ".something" found → ""
```

---

## Design Principles

1. **Never panic** — invalid queries, malformed content, and out-of-bounds indices all return empty results
2. **Prefix routing** — the query string declares the engine; no mode switching or builder patterns
3. **`>>` function pipe** — unambiguous across all engines; XPath `|` and JMESPath `|` work natively
4. **Batteries included** — post-processing, text pseudo-selectors, validators, and index selection are all built in

## Also Available

ChadSelect is also available as a [Python package](https://pypi.org/project/chadselect/) with identical API and query syntax.

## License

MIT
