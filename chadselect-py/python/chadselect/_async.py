"""
Async wrapper around :class:`ChadSelect`.

ChadSelect queries complete in 100µs–3ms (pure Rust, no I/O), so they
will not meaningfully block an ``asyncio`` event loop. This wrapper
provides an ``async`` interface for code-bases that are fully async and
want a consistent API.

For truly CPU-heavy workloads (e.g. processing thousands of documents),
use :func:`asyncio.to_thread` with a **separate** ``ChadSelect`` instance
created on the worker thread — this avoids cross-thread access to the
native object.

Requires Python 3.9+.
"""

from __future__ import annotations

from typing import List, Tuple

from chadselect._native import ChadSelect


class AsyncChadSelect:
    """Async-friendly wrapper around the Rust-powered ``ChadSelect``.

    Every query method is ``async`` for API consistency with async code.
    Under the hood, calls execute synchronously in Rust (100µs–3ms) —
    fast enough that event-loop blocking is negligible.

    Usage::

        cs = AsyncChadSelect()
        cs.add_html(html)                          # sync — instant
        price = await cs.select(0, "css:.price")   # async wrapper
    """

    __slots__ = ("_inner",)

    def __init__(self) -> None:
        self._inner = ChadSelect()

    # ── Content management (sync — these are instant) ────────────────────

    def add_text(self, content: str) -> None:
        """Add plain text content."""
        self._inner.add_text(content)

    def add_html(self, content: str) -> None:
        """Add HTML content (compatible with CSS, XPath, and Regex)."""
        self._inner.add_html(content)

    def add_json(self, content: str) -> None:
        """Add JSON content (compatible with JMESPath and Regex)."""
        self._inner.add_json(content)

    def content_count(self) -> int:
        """Return the number of loaded content items."""
        return self._inner.content_count()

    def clear(self) -> None:
        """Remove all loaded content."""
        self._inner.clear()

    # ── Querying (async wrappers) ────────────────────────────────────────

    async def query(self, index: int, query_str: str) -> List[str]:
        """Query all loaded content and return matching results."""
        return list(self._inner.query(index, query_str))

    async def select(self, index: int, query_str: str) -> str:
        """Return a single result string, or ``""`` on no match."""
        return self._inner.select(index, query_str)

    async def select_first(self, queries: List[Tuple[int, str]]) -> List[str]:
        """Try queries in order, return the first valid result set."""
        return list(self._inner.select_first(queries))

    async def select_many(self, queries: List[Tuple[int, str]]) -> List[str]:
        """Run multiple queries and return combined unique results."""
        return list(self._inner.select_many(queries))

    def __repr__(self) -> str:
        return f"AsyncChadSelect(content_count={self._inner.content_count()})"

    def __len__(self) -> int:
        return self._inner.content_count()
