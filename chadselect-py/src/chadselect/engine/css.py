"""CSS selector engine — powered by selectolax (lexbor).

Includes custom text pseudo-selectors that mirror the Rust crate:

- ``:has-text('...')``  — select elements whose subtree *contains* the text
- ``:contains-text('...')`` — select elements whose *own* text contains the substring
- ``:text-equals('...')``  — select elements whose text *equals* the argument
- ``:text-starts('...')``  — select elements whose text *starts with* the argument
- ``:text-ends('...')``    — select elements whose text *ends with* the argument
"""

from __future__ import annotations

import logging
import re as _re
from typing import List, Optional, Tuple

from selectolax.parser import HTMLParser, Node

from chadselect._functions import split_functions, parse_and_apply

logger = logging.getLogger(__name__)

# ── pseudo-selector regex ────────────────────────────────────────────────────
# Matches  :pseudo-name('argument')  (with optional trailing selectors)
_PSEUDO_RE = _re.compile(
    r":(?P<pseudo>has-text|contains-text|text-equals|text-starts|text-ends)"
    r"\(['\"](?P<arg>[^'\"]*)['\"]\)"
)


def _extract_pseudo(selector: str) -> Tuple[str, Optional[str], Optional[str], str]:
    """Split a selector into (base_selector, pseudo_name, pseudo_arg, trailing).

    Returns ``(selector, None, None, "")`` when no custom pseudo is present.
    """
    m = _PSEUDO_RE.search(selector)
    if not m:
        return selector, None, None, ""
    base = selector[: m.start()]
    trailing = selector[m.end() :].strip()  # e.g. " .value"
    return base, m.group("pseudo"), m.group("arg"), trailing


def _node_text(node: Node) -> str:
    """Full text content (subtree), stripped."""
    return (node.text(strip=True) or "").strip()


def _matches_pseudo(node: Node, pseudo: str, arg: str) -> bool:
    text = _node_text(node)
    if pseudo == "has-text":
        return arg in text
    if pseudo == "contains-text":
        return arg in text
    if pseudo == "text-equals":
        return text == arg
    if pseudo == "text-starts":
        return text.startswith(arg)
    if pseudo == "text-ends":
        return text.endswith(arg)
    return False


def process(selector_with_functions: str, html: str) -> List[str]:
    """Run a CSS selector against HTML content, with optional ``>>`` functions."""
    selector, func_chain = split_functions(selector_with_functions)

    # Check for get-attr in the function chain — need to handle before parsing
    attr_name = _extract_get_attr(func_chain)
    if attr_name:
        func_chain = _remove_get_attr(func_chain)

    tree = HTMLParser(html)

    # ── custom pseudo-selector handling ──────────────────────────────────
    base, pseudo, pseudo_arg, trailing = _extract_pseudo(selector)
    if pseudo:
        try:
            candidates = tree.css(base) if base else [tree.body]
        except Exception as e:
            logger.warning("CSS selector failed for '%s': %s", base, e)
            return []

        matched_nodes: List[Node] = []
        for node in candidates:
            if _matches_pseudo(node, pseudo, pseudo_arg):  # type: ignore[arg-type]
                if trailing:
                    # e.g. ":has-text('Exterior:') .value" — query inside
                    try:
                        matched_nodes.extend(node.css(trailing))
                    except Exception:
                        pass
                else:
                    matched_nodes.append(node)
        nodes = matched_nodes
    else:
        try:
            nodes = tree.css(selector)
        except Exception as e:
            logger.warning("CSS selector failed for '%s': %s", selector, e)
            return []

    # ── extract text / attributes ────────────────────────────────────────
    results: List[str] = []
    for node in nodes:
        if attr_name:
            val = node.attributes.get(attr_name, "")
            if val:
                results.append(val)
        else:
            text = node.text(strip=True)
            if text:
                results.append(text)

    if func_chain.strip():
        results = parse_and_apply(results, func_chain)

    return results


def _extract_get_attr(func_chain: str) -> str | None:
    """Extract the attribute name from a ``get-attr('name')`` call."""
    import re
    m = re.search(r"get-attr\(['\"](\w[\w-]*?)['\"]\)", func_chain)
    return m.group(1) if m else None


def _remove_get_attr(func_chain: str) -> str:
    """Remove ``get-attr(...)`` from a function chain string."""
    import re
    cleaned = re.sub(r"\s*get-attr\(['\"][\w-]+?['\"]\)\s*", " ", func_chain)
    # Clean up stray >> delimiters
    cleaned = re.sub(r"(>>\s*)+", ">> ", cleaned).strip().strip(">>").strip()
    return cleaned
