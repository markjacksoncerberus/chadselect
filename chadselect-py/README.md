# ChadSelect

Unified data extraction — CSS, XPath, Regex, and JMESPath behind one query interface.

```python
from chadselect import ChadSelect

cs = ChadSelect()
cs.add_html(html)
cs.add_json(json_str)

# One syntax, four engines
title = cs.select(0, "css:h1.title")
author = cs.select(0, "xpath://span[@class='author']/text()")
vin = cs.select(0, r"regex:[A-HJ-NPR-Z0-9]{17}")
name = cs.select(0, "json:data.products[0].name")

# Function piping
clean = cs.select(0, "css:.price >> trim >> uppercase()")
```

## Install

```bash
pip install chadselect
```

## Query Syntax

Queries use a `engine:expression` prefix:

| Prefix | Engine | Best For |
|--------|--------|----------|
| `css:` | CSS Selectors (selectolax) | HTML element selection |
| `xpath:` | XPath 1.0 (lxml) | Complex HTML/XML traversal |
| `regex:` | Regular Expressions (re) | Pattern matching on raw text |
| `json:` | JMESPath (jmespath) | JSON field extraction |

No prefix defaults to regex.

## Function Piping

Chain text transformations with `>>`:

```python
cs.select(0, "css:.price >> trim >> substring-after('$') >> uppercase()")
```

Available functions: `trim`, `uppercase()`, `lowercase()`, `normalize-space()`,
`substring-after('delim')`, `substring-before('delim')`, `substring(start, len)`,
`replace('old', 'new')`, `get-attr('name')`.

## API

```python
cs = ChadSelect()

# Load content
cs.add_html(html_string)
cs.add_json(json_string)
cs.add_text(plain_text)

# Query (index: 0=first, -1=all)
results = cs.query(-1, "css:.price")          # List[str] — all matches
value = cs.select(0, "css:.price")            # str — first match or ""

# Multi-query
first_hit = cs.select_first([(0, "css:#id"), (0, "xpath://fallback")])
combined = cs.select_many([(-1, "css:.a"), (-1, "css:.b")])

# Batch (fastest for many fields)
results = cs.query_batch([(-1, "css:.title"), (-1, "json:data.name")])

# With validators
results = cs.select_where(0, "css:.vin", lambda v: len(v) == 17)
```

## License

MIT
