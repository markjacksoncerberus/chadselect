# ChadSelect

**One query. Any format. Every selector.**

ChadSelect is a Rust data extraction library that unifies **Regex**, **XPath 1.0**, **CSS Selectors**, and **JMESPath** behind a single, dead-simple query interface. Load your content, prefix your query string, get results. That's it.

---

## Why ChadSelect?

Scraping and data extraction is messy. You're juggling `regex` for raw text, `scraper` for CSS, `sxd_xpath` for XPath, and `jmespath` for JSON — all with different APIs, different error handling, and different mental models.

ChadSelect collapses all of that into one struct and one query pattern:

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();
cs.add_html(html);
cs.add_json(json);

// CSS selector
cs.select(0, "css:.price");

// XPath
cs.select(0, "xpath://span[@id='vin']/text()");

// Regex (works on everything)
cs.select(0, r"regex:VIN:\s*(\w+)");

// JMESPath
cs.select(0, "json:inventory[0].price");
```

No separate parsers to manage. No error handling boilerplate. Queries that fail return empty strings — **never panics**.

---

## Features

- **Unified query API** — `regex:`, `xpath:`, `css:`, `json:` prefixes route to the right engine
- **Multi-content** — load multiple documents (HTML, JSON, plain text) and query across all of them
- **Post-processing functions** — pipe results through `normalize-space()`, `uppercase()`, `substring-after()`, and more
- **CSS text pseudo-selectors** — `:has-text()`, `:text-equals()`, `:text-starts()`, `:text-ends()`, `:contains-text()`
- **Index selection** — grab all results, the first, the Nth, or fallback through a priority list
- **Lazy caching** — parsed documents are cached on first query; subsequent queries reuse them
- **Zero panics** — every code path returns empty results on failure, never crashes

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
chadselect = "0.1.1"
```

---

## Quick Start

```rust
use chadselect::ChadSelect;

let mut cs = ChadSelect::new();

// Load content
cs.add_html(r#"<div class="product"><span class="price">$49.99</span></div>"#.to_string());

// Query it
let price = cs.select(0, "css:.price");
assert_eq!(price, "$49.99");
```

---

## Query Prefixes

Every query string starts with a prefix that tells ChadSelect which engine to use:

| Prefix    | Engine   | Content Types | Example |
|-----------|----------|---------------|---------|
| `regex:`  | Regex    | All           | `regex:price:\s*\$(\d+)` |
| `xpath:`  | XPath 1.0| HTML, Text    | `xpath://span[@class='vin']/text()` |
| `css:`    | CSS      | HTML          | `css:div.product > .price` |
| `json:`   | JMESPath | JSON          | `json:store.inventory[0].name` |

If no prefix is provided, the query defaults to **regex**.

---

## API

### Loading Content

```rust
let mut cs = ChadSelect::new();

cs.add_html(html_string);   // HTML content (works with CSS, XPath, Regex)
cs.add_json(json_string);   // JSON content (works with JMESPath, Regex)
cs.add_text(text_string);   // Plain text   (works with Regex, XPath)

cs.content_count();          // Number of loaded documents
cs.clear();                  // Remove all content
```

### Querying

#### `select(index, query) -> String`

Returns a single result string. Empty string if nothing matches.

```rust
// First match
cs.select(0, "css:.title");

// Second match
cs.select(1, "css:.title");
```

#### `query(index, query, content_type_override) -> Vec<String>`

Returns all matches as a vector. Use index `-1` for all results.

```rust
// All matches
let all = cs.query(-1, "css:.price", None);

// Specific index
let first = cs.query(0, "css:.price", None);

// Override content type
let result = cs.query(0, r"regex:\d+", Some(ContentType::Html));
```

#### `select_first(queries) -> Vec<String>`

Tries multiple queries in order, returns results from the first one that matches. Perfect for fallback chains.

```rust
let result = cs.select_first(vec![
    (0, "css:#exact-id"),                       // Try this first
    (0, "xpath://span[@class='alt']/text()"),   // Then this
    (0, r"regex:fallback:\s*(.+)"),             // Last resort
]);
```

#### `select_many(queries) -> Vec<String>`

Runs multiple queries and combines all unique results.

```rust
let results = cs.select_many(vec![
    (-1, "css:.primary-price"),
    (-1, "css:.sale-price"),
    (-1, "xpath://span[@class='msrp']/text()"),
]);
```

---

## Post-Processing Functions

Pipe query results through text functions using `>>`. Works with both CSS and XPath queries.

We use `>>` instead of `|` because `|` is the union operator in XPath 1.0 and a pipe in JMESPath. The `>>` delimiter is unambiguous across all selector engines.

```
css:.selector >> function1() >> function2()
xpath://path/text() >> function1() >> function2()
```

### Available Functions

| Function | Description | Example |
|----------|-------------|---------|
| `normalize-space()` | Trim + collapse internal whitespace | `css:.desc >> normalize-space()` |
| `trim()` | Trim leading/trailing whitespace | `css:.title >> trim()` |
| `uppercase()` | Convert to uppercase | `css:.vin >> uppercase()` |
| `lowercase()` | Convert to lowercase | `css:.name >> lowercase()` |
| `substring(start, len)` | Extract substring (0-indexed) | `css:.code >> substring(0, 3)` |
| `substring-after('delim')` | Text after delimiter | `css:.vin >> substring-after('VIN: ')` |
| `substring-before('delim')` | Text before delimiter | `css:.info >> substring-before(': ')` |
| `replace('find', 'repl')` | String replacement | `css:.price >> replace('$', 'USD ')` |
| `get-attr('name')` | Extract element attribute (CSS only) | `css:a >> get-attr('href')` |

### Chaining Example

```rust
// Extract VIN from "VIN: 1HGCM82633A123456", take first 3 chars, lowercase
let result = cs.select(0, "css:.vin >> substring-after('VIN: ') >> substring(0, 3) >> lowercase()");
// => "1hg"
```

---

## CSS Text Pseudo-Selectors

Custom pseudo-selectors for filtering elements by their text content. These go beyond standard CSS to let you match elements based on what they contain.

| Pseudo-Selector | Behavior |
|-----------------|----------|
| `:has-text('x')` | Element or its descendants contain the text |
| `:contains-text('x')` | Element's own text content contains the text |
| `:text-equals('x')` | Element's text content exactly equals the text |
| `:text-starts('x')` | Element's text content starts with the text |
| `:text-ends('x')` | Element's text content ends with the text |

### Usage

```rust
cs.add_html(r#"
    <div class="item">
        <div class="label">Exterior:</div>
        <div class="value">Blue Metallic</div>
    </div>
    <div class="item">
        <div class="label">Interior:</div>
        <div class="value">Black Leather</div>
    </div>
"#.to_string());

// Find .value inside the .item that contains "Exterior:"
let color = cs.select(0, "css:.item:has-text('Exterior:') .value");
// => "Blue Metallic"

// Find values ending with a specific word
let result = cs.select(0, "css:.value:text-ends('Leather')");
// => "Black Leather"

// Combine with post-processing
let result = cs.select(0, "css:.item:has-text('Interior:') .value >> uppercase()");
// => "BLACK LEATHER"
```

---

## Regex

Regex queries work on all content types. Capture groups are extracted automatically.

```rust
cs.add_text(r#"vehicleLat":"40.7128""#.to_string());

// With capture group — returns the captured value
let lat = cs.select(0, r#"regex:vehicleLat":"([0-9.]+)""#);
// => "40.7128"

// Without capture group — returns full match
let prices = cs.query(-1, r"regex:\$\d+", None);
// => ["$100", "$200", "$300"]
```

---

## XPath 1.0

Full XPath 1.0 support for HTML documents, powered by `sxd_html` + `sxd_xpath`. The `|` union operator works natively since function pipes use `>>`.

```rust
cs.add_html(html.to_string());

// Text extraction
cs.select(0, "xpath://h1/text()");

// Attribute-based selection
cs.select(0, "xpath://span[@id='vin']/text()");

// XPath functions
cs.select(0, "xpath:normalize-space(//h1)");
cs.select(0, "xpath:string(//span[@id='vin'])");

// XPath union operator (|) works without conflict
cs.query(-1, "xpath://h1/text() | //h2/text()", None);

// With post-processing via >>
cs.select(0, "xpath://div[@class='vin']/text() >> substring-after('VIN: ')");
```

---

## JMESPath

Query JSON documents using [JMESPath](https://jmespath.org/) expressions.

```rust
cs.add_json(r#"{
    "store": {
        "inventory": [
            {"name": "Widget", "price": 25},
            {"name": "Gadget", "price": 50}
        ]
    }
}"#.to_string());

// Simple path
let name = cs.select(0, "json:store.inventory[0].name");
// => "Widget"

// Array projection
let names = cs.query(-1, "json:store.inventory[].name", None);
// => ["Widget", "Gadget"]
```

---

## Multi-Content Queries

Load multiple documents and query across all of them simultaneously.

```rust
let mut cs = ChadSelect::new();
cs.add_text("price: $100".to_string());
cs.add_text("price: $200".to_string());
cs.add_html("<span>price: $300</span>".to_string());

// Regex searches across all loaded content
let prices = cs.query(-1, r"regex:\$(\d+)", None);
// => ["100", "200", "300"]
```

---

## Project Structure

```
src/
├── lib.rs              # Public API — ChadSelect struct and re-exports
├── content.rs          # ContentItem, ContentType, lazy caching
├── query.rs            # Query parsing, prefix routing, compatibility
├── functions.rs        # Post-processing text functions (>> pipeline)
└── engine/
    ├── mod.rs
    ├── regex.rs        # Regex extraction engine
    ├── xpath.rs        # XPath 1.0 extraction engine
    ├── css.rs          # CSS selector engine + text pseudo-selectors
    └── json.rs         # JMESPath extraction engine
tests/
├── regex_tests.rs      # Regex engine tests
├── xpath_tests.rs      # XPath engine tests
├── css_tests.rs        # CSS engine tests
├── json_tests.rs       # JMESPath engine tests
├── functions_tests.rs  # Post-processing function tests
└── integration_tests.rs# Cross-engine and multi-content tests
```

---

## Design Principles

1. **Never panic** — Invalid queries, malformed content, out-of-bounds indices: everything returns empty results.
2. **Prefix routing** — The query string itself declares the engine. No mode switching, no builder patterns.
3. **`>>` function pipe** — Unambiguous across all engines. XPath `|` union and JMESPath `|` pipe work natively.
4. **Lazy & cached** — Documents are parsed once on first access, then reused. XPath factories and contexts are cached.
5. **Batteries included** — Post-processing, text pseudo-selectors, and index selection are built in. No external pipeline needed.

---

## Crate Dependencies

| Crate | Purpose |
|-------|---------|
| [`regex`](https://crates.io/crates/regex) | Regular expressions |
| [`sxd-document`](https://crates.io/crates/sxd-document) + [`sxd-xpath`](https://crates.io/crates/sxd-xpath) | XPath 1.0 evaluation |
| [`sxd-html`](https://crates.io/crates/sxd-html) | HTML → XPath document parsing |
| [`scraper`](https://crates.io/crates/scraper) | CSS selector engine |
| [`serde_json`](https://crates.io/crates/serde_json) | JSON parsing |
| [`jmespath`](https://crates.io/crates/jmespath) | JMESPath evaluation |
| [`log`](https://crates.io/crates/log) | Structured logging (no output without a subscriber) |

---

## License

MIT OR Apache-2.0
