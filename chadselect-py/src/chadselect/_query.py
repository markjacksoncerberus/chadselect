"""
Query type parsing — prefix-based routing to the correct extraction engine.

Mirrors the Rust crate's ``query.rs`` exactly.
"""

from __future__ import annotations

from enum import Enum, auto
from typing import Tuple

#: The function-pipe delimiter used to separate a selector expression from its
#: post-processing function chain.
#:
#: We use ``>>`` instead of ``|`` because ``|`` is a union operator in XPath 1.0
#: and a pipe operator in JMESPath, which would create ambiguity.
FUNCTION_PIPE: str = ">>"


class QueryType(Enum):
    """Parsed query engine type."""

    #: Regex pattern — works on all content types.
    REGEX = auto()
    #: XPath 1.0 expression — works on HTML and Text.
    XPATH = auto()
    #: JMESPath expression — works on JSON.
    JSON = auto()
    #: CSS selector — works on HTML.
    CSS = auto()


#: Content type identifiers.
class ContentType(Enum):
    TEXT = auto()
    HTML = auto()
    JSON = auto()


#: Maps query types to compatible content types.
_COMPAT = {
    QueryType.REGEX: {ContentType.TEXT, ContentType.HTML, ContentType.JSON},
    QueryType.XPATH: {ContentType.TEXT, ContentType.HTML},
    QueryType.JSON: {ContentType.JSON},
    QueryType.CSS: {ContentType.HTML},
}


def parse_query(query: str) -> Tuple[QueryType, str]:
    """Parse a prefixed query string into ``(QueryType, expression)``.

    Supported prefixes: ``regex:``, ``xpath:``, ``json:``, ``css:``.
    No prefix defaults to Regex.
    """
    if query.startswith("regex:"):
        return QueryType.REGEX, query[6:]
    if query.startswith("json:"):
        return QueryType.JSON, query[5:]
    if query.startswith("xpath:"):
        return QueryType.XPATH, query[6:]
    if query.startswith("css:"):
        return QueryType.CSS, query[4:]
    # Default to regex
    return QueryType.REGEX, query


def is_query_compatible(query_type: QueryType, content_type: ContentType) -> bool:
    """Check whether a query type can run against a content type."""
    return content_type in _COMPAT[query_type]
