"""
ChadSelect — the main extraction class.

API-compatible with the Rust ``chadselect`` crate.
"""

from __future__ import annotations

import logging
from typing import Any, Callable, List, Optional, Sequence, Tuple

from chadselect._query import ContentType, QueryType, parse_query, is_query_compatible
from chadselect.engine import css as css_engine
from chadselect.engine import xpath as xpath_engine
from chadselect.engine import regex as regex_engine
from chadselect.engine import json as json_engine

logger = logging.getLogger(__name__)


def _default_valid(s: str) -> bool:
    """Default validator — non-empty, non-whitespace."""
    return bool(s and s.strip())


class _ContentItem:
    """Internal content item with type tag."""

    __slots__ = ("content", "content_type")

    def __init__(self, content: str, content_type: ContentType) -> None:
        self.content = content
        self.content_type = content_type


class ChadSelect:
    """Unified data extraction — CSS, XPath, Regex, and JMESPath.

    Load content, then query with a prefixed query string::

        cs = ChadSelect()
        cs.add_html('<span class="price">$49.99</span>')
        price = cs.select(0, "css:.price")  # "$49.99"

    Query prefixes:
        - ``css:``   → CSS Selectors (selectolax / lexbor)
        - ``xpath:`` → XPath 1.0 (lxml / libxml2)
        - ``json:``  → JMESPath
        - ``regex:`` → Python ``re``
        - *(none)*   → Regex (default)

    Post-processing via ``>>``::

        cs.select(0, "css:.price >> normalize-space() >> uppercase()")
    """

    __slots__ = ("_content_list",)

    def __init__(self) -> None:
        self._content_list: List[_ContentItem] = []

    # ── Content management ──────────────────────────────────────────────

    def add_text(self, content: str) -> None:
        """Add plain text content."""
        self._content_list.append(_ContentItem(content, ContentType.TEXT))

    def add_html(self, content: str) -> None:
        """Add HTML content (compatible with CSS, XPath, and Regex)."""
        self._content_list.append(_ContentItem(content, ContentType.HTML))

    def add_json(self, content: str) -> None:
        """Add JSON content (compatible with JMESPath and Regex)."""
        self._content_list.append(_ContentItem(content, ContentType.JSON))

    def content_count(self) -> int:
        """Return the number of loaded content items."""
        return len(self._content_list)

    def clear(self) -> None:
        """Remove all loaded content."""
        self._content_list.clear()

    # ── Querying ────────────────────────────────────────────────────────

    def query(self, index: int, query_str: str) -> List[str]:
        """Query all loaded content and return matching results.

        Args:
            index: ``-1`` returns **all** matches. ``>= 0`` returns the
                match at that position (or empty list if out of bounds).
            query_str: Prefixed query string (e.g. ``"css:.price"``).

        Returns:
            List of matched strings. Never raises — invalid queries or
            out-of-bounds indices return ``[]``.
        """
        query_type, expression = parse_query(query_str)

        all_results: List[str] = []

        for item in self._content_list:
            if not is_query_compatible(query_type, item.content_type):
                continue

            if query_type == QueryType.CSS:
                results = css_engine.process(expression, item.content)
            elif query_type == QueryType.XPATH:
                results = xpath_engine.process(expression, item.content)
            elif query_type == QueryType.REGEX:
                results = regex_engine.process(expression, item.content)
            elif query_type == QueryType.JSON:
                results = json_engine.process(expression, item.content)
            else:
                results = []

            all_results.extend(results)

        return _select_by_index(all_results, index)

    def select(self, index: int, query_str: str) -> str:
        """Return a single result string (the first match), or ``""``.

        A result is valid when it is non-empty and non-whitespace.
        """
        return self.select_where(index, query_str, _default_valid)

    def select_where(
        self,
        index: int,
        query_str: str,
        valid: Callable[[str], bool],
    ) -> str:
        """Like :meth:`select` but with a custom validity check.

        Args:
            valid: Receives each candidate string, returns ``True`` to accept.
        """
        result = self.query(index, query_str)
        if result and valid(result[0]):
            return result[0]
        return ""

    def select_first(
        self, queries: Sequence[Tuple[int, str]]
    ) -> List[str]:
        """Try multiple queries in order, return the first valid result set.

        A result set is valid when all its elements are non-empty and
        non-whitespace.
        """
        return self.select_first_where(queries, _default_valid)

    def select_first_where(
        self,
        queries: Sequence[Tuple[int, str]],
        valid: Callable[[str], bool],
    ) -> List[str]:
        """Like :meth:`select_first` but with a custom validity check."""
        for index, query_str in queries:
            result = self.query(index, query_str)
            if result and all(valid(r) for r in result):
                return result
        return []

    def select_many(
        self, queries: Sequence[Tuple[int, str]]
    ) -> List[str]:
        """Run multiple queries and return combined unique results."""
        return self.select_many_where(queries, _default_valid)

    def select_many_where(
        self,
        queries: Sequence[Tuple[int, str]],
        valid: Callable[[str], bool],
    ) -> List[str]:
        """Like :meth:`select_many` but with a custom validity check."""
        seen: set[str] = set()
        out: List[str] = []
        for index, query_str in queries:
            for r in self.query(index, query_str):
                if valid(r) and r not in seen:
                    seen.add(r)
                    out.append(r)
        return out

    def query_batch(
        self, queries: Sequence[Tuple[int, str]]
    ) -> List[List[str]]:
        """Execute multiple queries in one call.

        Returns a list of result lists, one per input query, in order.
        This is the most efficient way to extract many fields.
        """
        return [self.query(index, q) for index, q in queries]

    # ── Dunder ──────────────────────────────────────────────────────────

    def __repr__(self) -> str:
        return f"ChadSelect(content_count={self.content_count()})"

    def __len__(self) -> int:
        return self.content_count()


def _select_by_index(results: List[str], index: int) -> List[str]:
    """Select results by index — ``-1`` means 'all'."""
    if index == -1:
        return results
    if index >= 0:
        if index < len(results):
            return [results[index]]
        logger.warning(
            "Index %d out of range (have %d results)", index, len(results)
        )
        return []
    logger.warning("Invalid index: %d", index)
    return []
