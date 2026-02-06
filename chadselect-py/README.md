# chadselect (Python)

Rust-powered unified data extraction — Regex, XPath 1.0, CSS Selectors, and JMESPath behind one query interface.

This is the Python binding for [chadselect](https://github.com/markjacksoncerberus/chadselect), built with [PyO3](https://pyo3.rs) and [maturin](https://www.maturin.rs).

## Install

```bash
pip install chadselect
```

## Quick Start

```python
from chadselect import ChadSelect

cs = ChadSelect()
cs.add_html('<span class="price">$49.99</span>')

price = cs.select(0, "css:.price")          # "$49.99"
results = cs.query(-1, "regex:\\$[\\d.]+")   # ["$49.99"]
```

### Async

```python
from chadselect import AsyncChadSelect

cs = AsyncChadSelect()
cs.add_html(html)
price = await cs.select(0, "css:.price")
```

## Query Prefixes

| Prefix   | Engine         | Content Types      |
|----------|----------------|--------------------|
| `css:`   | CSS Selectors  | HTML               |
| `xpath:` | XPath 1.0      | HTML               |
| `json:`  | JMESPath       | JSON               |
| `regex:` | Regex          | HTML, JSON, Text   |
| *(none)* | Regex          | HTML, JSON, Text   |

## Post-Processing Functions

Chain functions with `>>`:

```python
cs.select(0, "css:.price >> trim >> uppercase")
```

Available: `trim`, `uppercase`, `lowercase`, `normalize-space`, `substring(start,len)`, `substring-after(delim)`, `substring-before(delim)`, `replace(old,new)`, `get-attr(name)`.

## License

MIT
