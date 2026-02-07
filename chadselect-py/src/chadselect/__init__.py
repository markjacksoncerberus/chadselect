"""
ChadSelect — Unified data extraction.

CSS Selectors, XPath 1.0, Regex, and JMESPath behind one query interface
with chainable post-processing functions.

Usage::

    from chadselect import ChadSelect

    cs = ChadSelect()
    cs.add_html('<span class="price">$49.99</span>')
    price = cs.select(0, "css:.price")
    # "$49.99"

Query prefixes::

    css:       → CSS Selectors (selectolax/lexbor)
    xpath:     → XPath 1.0 (lxml/libxml2)
    json:      → JMESPath
    regex:     → Regex (re stdlib)
    (no prefix) → Regex (default)

Post-processing functions (pipe with >>)::

    cs.select(0, "css:.price >> normalize-space() >> uppercase()")
"""

from chadselect._chadselect import ChadSelect
from chadselect._query import FUNCTION_PIPE, QueryType, parse_query
from chadselect._functions import supported_text_functions

__all__ = [
    "ChadSelect",
    "FUNCTION_PIPE",
    "QueryType",
    "parse_query",
    "supported_text_functions",
]
__version__ = "0.2.0"
