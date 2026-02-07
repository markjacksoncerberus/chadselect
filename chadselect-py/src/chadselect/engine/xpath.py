"""XPath 1.0 engine — powered by lxml (libxml2)."""

from __future__ import annotations

import logging
from typing import List

from lxml import html as lxml_html

from chadselect._functions import split_functions, parse_and_apply

logger = logging.getLogger(__name__)


def process(xpath_with_functions: str, content: str) -> List[str]:
    """Run an XPath 1.0 expression against HTML/text content.

    Supports ``>>`` function piping.
    """
    xpath_expr, func_chain = split_functions(xpath_with_functions)

    try:
        tree = lxml_html.fromstring(content)
        raw = tree.xpath(xpath_expr)
    except Exception as e:
        logger.warning("XPath failed for '%s': %s", xpath_expr, e)
        return []

    # XPath can return a plain string (e.g. normalize-space(), string())
    # instead of a list.  Wrap it so the loop below works correctly.
    if isinstance(raw, str):
        raw = [raw]

    results: List[str] = []
    for item in raw:
        if hasattr(item, "text_content"):
            # It's an Element
            text = item.text_content().strip()
        else:
            # It's a string (text node or attribute)
            text = str(item).strip()
        if text:
            results.append(text)

    if func_chain.strip():
        results = parse_and_apply(results, func_chain)

    return results
