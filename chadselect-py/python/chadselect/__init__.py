"""
ChadSelect — Unified data extraction powered by Rust.

Regex, XPath 1.0, CSS Selectors, and JMESPath behind one interface.
Queries complete in 100µs–3ms (pure Rust, no I/O).

Usage::

    from chadselect import ChadSelect

    cs = ChadSelect()
    cs.add_html('<span class="price">$49.99</span>')
    price = cs.select(0, "css:.price")

For async usage::

    from chadselect import AsyncChadSelect

    cs = AsyncChadSelect()
    cs.add_html(html)
    price = await cs.select(0, "css:.price")
"""

from chadselect._native import ChadSelect
from chadselect._async import AsyncChadSelect

__all__ = ["ChadSelect", "AsyncChadSelect"]
__version__ = "0.1.0"
