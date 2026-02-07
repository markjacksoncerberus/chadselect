# ChadSelect

**One query. Any format. Every selector.**

Unified data extraction — **CSS Selectors**, **XPath 1.0**, **Regex**, and **JMESPath** behind one query interface. Load your content, prefix your query, get results. Never raises.

[![PyPI](https://img.shields.io/pypi/v/chadselect.svg)](https://pypi.org/project/chadselect/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

```python
from chadselect import ChadSelect

cs = ChadSelect()
cs.add_html('<span class="price">$49.99</span>')

price = cs.select(0, "css:.price")
assert price == "$49.99"
```

## Install

```bash
pip install chadselect
```

---

## Query Syntax

Every query uses an `engine:expression` prefix. No prefix defaults to regex.

| Prefix | Engine | Content Types | Backed By |
|--------|--------|---------------|-----------|
| `css:` | CSS Selectors | HTML | [selectolax](https://pypi.org/project/selectolax/) (lexbor) |
| `xpath:` | XPath 1.0 | HTML, Text | [lxml](https://pypi.org/project/lxml/) (libxml2) |
| `regex:` | Regular Expressions | All | [re](https://docs.python.org/3/library/re.html) (stdlib) |
| `json:` | JMESPath | JSON | [jmespath](https://pypi.org/project/jmespath/) |

---

## The `index` Parameter

Every query method takes an `index` argument that controls **which match** to return:

| Value | Behavior |
|-------|----------|
| `-1` | Return **all** matches across every loaded document |
| `0` | Return only the **first** match |
| `N` | Return only the **Nth** match (0-based) |

```python
cs = ChadSelect()
cs.add_html("<ul><li>A</li><li>B</li><li>C</li></ul>")

all_items = cs.query(-1, "css:li")  # ["A", "B", "C"]
first     = cs.query(0,  "css:li")  # ["A"]
third     = cs.query(2,  "css:li")  # ["C"]
oob       = cs.query(99, "css:li")  # []  (out of bounds — never raises)

# select() wraps query() — returns a single str
s = cs.select(0, "css:li")           # "A"
s = cs.select(-1, "css:li")          # "A" (first of all matches)
```

When multiple documents are loaded, `-1` aggregates results from **all** compatible documents before indexing.

---

## Content Management

Load one or more documents. Each document is tagged by type and only queried by compatible engines.

```python
from chadselect import ChadSelect

cs = ChadSelect()

# HTML — compatible with css:, xpath:, regex:
cs.add_html("""
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
""")

# JSON — compatible with json:, regex:
cs.add_json("""{
  "inventory": [
    {"id": 1, "name": "Civic",   "price": 28500, "tags": ["sedan", "honda"]},
    {"id": 2, "name": "Accord",  "price": 34000, "tags": ["sedan", "honda"]},
    {"id": 3, "name": "CR-V",    "price": 32500, "tags": ["suv",   "honda"]}
  ],
  "dealer": {"name": "Metro Honda", "rating": 4.8}
}""")

# Plain text — compatible with regex:, xpath:
cs.add_text("Order #12345 confirmed. Total: $99.50")

assert cs.content_count() == 3

cs.clear()  # remove all content
```

---

## CSS Selectors

Standard CSS selectors, plus custom text pseudo-selectors for scraping.

```python
cs = ChadSelect()
cs.add_html("""
<ul class="products">
  <li class="product" data-id="1"><span class="name">Widget</span><span class="price">$19.99</span></li>
  <li class="product" data-id="2"><span class="name">Gadget</span><span class="price">$49.99</span></li>
  <li class="product" data-id="3"><span class="name">Doohickey</span><span class="price">$9.99</span></li>
</ul>
""")

# Basic selectors
first_name = cs.select(0, "css:.product .name")
assert first_name == "Widget"

# All matches — index -1
all_prices = cs.query(-1, "css:.product .price")
assert all_prices == ["$19.99", "$49.99", "$9.99"]

# Nth match — index 2 (0-based)
third = cs.query(2, "css:.product .name")
assert third == ["Doohickey"]

# Attribute extraction via get-attr()
id_val = cs.select(0, "css:.product >> get-attr('data-id')")
assert id_val == "1"
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

```python
cs = ChadSelect()
cs.add_html("""
<div class="specs">
  <div class="row"><span class="label">Exterior</span><span class="value">Blue Metallic</span></div>
  <div class="row"><span class="label">Interior</span><span class="value">Black Leather</span></div>
  <div class="row"><span class="label">Engine</span><span class="value">2.0L Turbo</span></div>
</div>
""")

# :has-text — matches the .row whose subtree contains "Exterior"
color = cs.select(0, "css:.row:has-text('Exterior') .value")
assert color == "Blue Metallic"

# :text-equals — exact match on element text
engine_label = cs.select(0, "css:.label:text-equals('Engine')")
assert engine_label == "Engine"

# :text-starts — prefix match
starts_e = cs.select(0, "css:.label:text-starts('Ext')")
assert starts_e == "Exterior"

# :text-ends — suffix match
ends_or = cs.select(0, "css:.label:text-ends('ior')")
assert ends_or == "Exterior"

# Combine with function piping
upper_interior = cs.select(0, "css:.row:has-text('Interior') .value >> uppercase()")
assert upper_interior == "BLACK LEATHER"
```

---

## XPath 1.0

Full XPath 1.0 support including axes, predicates, and XPath functions.

```python
cs = ChadSelect()
cs.add_html("""
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
""")

# text() extraction
title = cs.select(0, "xpath://h1[@id='title']/text()")
assert title == "  2024 Honda Civic  "

# With normalize-space
clean_title = cs.select(0, "xpath:normalize-space(//h1[@id='title'])")
assert clean_title == "2024 Honda Civic"

# Predicate-based selection — find the <td> after "VIN"
vin = cs.select(0, "xpath://tr[td='VIN']/td[2]/text()")
assert vin == "1HGFE2F59PA000001"

# All values from the second column
all_values = cs.query(-1, "xpath://table[@class='specs']//tr/td[2]/text()")
assert all_values == ["1HGFE2F59PA000001", "$28,500", "12,345 mi"]

# XPath string() on attribute
title_id = cs.select(0, "xpath:string(//h1/@id)")
assert title_id == "title"
```

---

## Regex

Capture groups or full matches. Works on HTML, JSON, and plain text content.

```python
cs = ChadSelect()
cs.add_text("VIN: 1HGFE2F59PA000001 | Stock #: A12345 | Price: $28,500")

# Capture group — returns the group, not the full match
vin = cs.select(0, r"regex:VIN:\s*([A-HJ-NPR-Z0-9]{17})")
assert vin == "1HGFE2F59PA000001"

# Full match — no capture group
stock = cs.select(0, r"regex:Stock #:\s*\S+")
assert stock == "Stock #: A12345"

# Multiple capture groups — returns first group
price_digits = cs.select(0, r"regex:Price:\s*\$([0-9,]+)")
assert price_digits == "28,500"

# All matches
all_numbers = cs.query(-1, r"regex:\d+")
# Returns all digit sequences found in the text

# No prefix — defaults to regex
vin2 = cs.select(0, r"[A-HJ-NPR-Z0-9]{17}")
assert vin2 == "1HGFE2F59PA000001"
```

### Regex on HTML

Regex runs on the raw HTML string, not parsed text — useful for extracting from attributes, comments, or script tags.

```python
cs = ChadSelect()
cs.add_html("<script>var price = 28500;</script>")

price = cs.select(0, r"regex:var price\s*=\s*(\d+)")
assert price == "28500"
```

---

## JMESPath (JSON)

Full JMESPath expression support for structured JSON extraction.

```python
cs = ChadSelect()
cs.add_json("""{
  "inventory": [
    {"id": 1, "name": "Civic",   "price": 28500, "tags": ["sedan", "honda"]},
    {"id": 2, "name": "Accord",  "price": 34000, "tags": ["sedan", "honda"]},
    {"id": 3, "name": "CR-V",    "price": 32500, "tags": ["suv",   "honda"]}
  ],
  "dealer": {"name": "Metro Honda", "rating": 4.8}
}""")

# Simple field access
dealer = cs.select(0, "json:dealer.name")
assert dealer == "Metro Honda"

# Array indexing
first = cs.select(0, "json:inventory[0].name")
assert first == "Civic"

# Projection — all names
names = cs.query(-1, "json:inventory[*].name")
assert names == ["Civic", "Accord", "CR-V"]

# Filter expression
expensive = cs.query(-1, "json:inventory[?price > `30000`].name")
assert expensive == ["Accord", "CR-V"]

# Nested access
rating = cs.select(0, "json:dealer.rating")
assert rating == "4.8"

# Flatten nested arrays
all_tags = cs.query(-1, "json:inventory[*].tags[]")
assert all_tags == ["sedan", "honda", "sedan", "honda", "suv", "honda"]
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

```python
cs = ChadSelect()
cs.add_html('<div class="info">  VIN: 1HGFE2F59PA000001  </div>')

# Chain: extract text → get everything after "VIN: " → first 3 chars → lowercase
result = cs.select(0, "css:.info >> substring-after('VIN: ') >> substring(0, 3) >> lowercase()")
assert result == "1hg"
```

```python
cs = ChadSelect()
cs.add_html('<a class="link" href="/inventory/123">View Car</a>')

# Attribute extraction
href = cs.select(0, "css:a.link >> get-attr('href')")
assert href == "/inventory/123"
```

```python
cs = ChadSelect()
cs.add_html('<span class="price">  $ 28,500  </span>')

# Clean + transform
clean_price = cs.select(0, "css:.price >> normalize-space() >> replace('$ ', '$')")
assert clean_price == "$28,500"
```

---

## API Reference

### Core Query Methods

```python
from chadselect import ChadSelect

cs = ChadSelect()
cs.add_html(html)

# query() — returns list[str], never raises
all_matches = cs.query(-1, "css:.price")   # all results
first_only  = cs.query(0,  "css:.price")   # list with 1st result or []
third       = cs.query(2,  "css:.price")   # list with 3rd result or []

# select() — returns str, empty on no match
price = cs.select(0, "css:.price")          # first valid result or ""
```

### Fallback Chains — `select_first`

Try queries in priority order. Returns the first result set where all values pass validation.

```python
cs = ChadSelect()
cs.add_html('<span class="alt-price">$28,500</span>')

# #exact-id doesn't exist, falls through to .alt-price
result = cs.select_first([
    (0, "css:#exact-id"),
    (0, "css:.alt-price"),
    (0, r"regex:\$[\d,]+"),
])
assert result == ["$28,500"]
```

### Multi-Source — `select_many`

Combine unique results from multiple queries.

```python
cs = ChadSelect()
cs.add_html("""
<span class="msrp">$30,000</span>
<span class="sale">$28,500</span>
""")

prices = cs.select_many([
    (0, "css:.msrp"),
    (0, "css:.sale"),
])
# Contains both "$30,000" and "$28,500" (unique, order preserved)
assert "$30,000" in prices
assert "$28,500" in prices
```

### Custom Validators — `select_where`

Filter results with a callback. The `_where` variants exist for `select`, `select_first`, and `select_many`.

```python
cs = ChadSelect()
cs.add_html('<span class="price">0</span><span class="price">28500</span>')

# Reject "0" as a valid price
price = cs.select_where(0, "css:.price", lambda s: s != "0")
assert price == ""  # first match "0" rejected, no fallback within select_where

# With select_first_where — falls through to next query
cs2 = ChadSelect()
cs2.add_text("a: 0\nb: 42")

r = cs2.select_first_where(
    [(0, r"a: (\d+)"), (0, r"b: (\d+)")],
    lambda s: s != "0",
)
assert r == ["42"]
```

### Batch Queries — `query_batch`

Execute many queries in one call. Returns `list[list[str]]` in input order.

```python
cs = ChadSelect()
cs.add_html("<h1>Civic</h1><span class='price'>$28,500</span>")
cs.add_json('{"dealer": "Metro Honda"}')

results = cs.query_batch([
    (0, "css:h1"),
    (0, "css:.price"),
    (0, "json:dealer"),
])
assert results[0] == ["Civic"]
assert results[1] == ["$28,500"]
assert results[2] == ["Metro Honda"]
```

---

## Multi-Content Queries

When multiple documents are loaded, queries search across all compatible content. Use `query(-1, ...)` to get results from every document.

```python
cs = ChadSelect()

cs.add_html('<span class="title">Page 1</span>')
cs.add_html('<span class="title">Page 2</span>')

# Searches both HTML documents
titles = cs.query(-1, "css:.title")
assert titles == ["Page 1", "Page 2"]

# Mixing content types
cs.add_json('{"title": "JSON Title"}')

# css: only queries HTML content — JSON is skipped
html_titles = cs.query(-1, "css:.title")
assert html_titles == ["Page 1", "Page 2"]

# json: only queries JSON content
json_title = cs.select(0, "json:title")
assert json_title == "JSON Title"

# regex: searches everything
all_results = cs.query(-1, r"regex:(?:Page \d|JSON Title)")
assert len(all_results) == 3
```

---

## Error Handling

ChadSelect **never raises**. Every invalid query, malformed content, or out-of-bounds index returns empty results.

```python
cs = ChadSelect()
cs.add_html("<div>hello</div>")

# Invalid CSS selector — returns ""
r = cs.select(0, "css:][invalid")
assert r == ""

# Out of bounds index — returns []
r = cs.query(999, "css:div")
assert r == []

# Wrong engine for content type — returns ""
cs.add_json('{"a": 1}')
r = cs.select(0, "css:.something")  # css: doesn't apply to JSON
# Only the HTML is searched, no ".something" found → ""
```

---

## Design Principles

1. **Never raise** — invalid queries, malformed content, and out-of-bounds indices all return empty results
2. **Prefix routing** — the query string declares the engine; no mode switching or builder patterns
3. **`>>` function pipe** — unambiguous across all engines; XPath `|` and JMESPath `|` work natively
4. **Batteries included** — post-processing, text pseudo-selectors, validators, and index selection are all built in

## Also Available

ChadSelect is also available as a [Rust crate](https://crates.io/crates/chadselect) with identical API and query syntax.

## License

MIT
