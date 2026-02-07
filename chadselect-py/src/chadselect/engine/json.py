"""JMESPath engine — powered by the ``jmespath`` library."""

from __future__ import annotations

import json
import logging
from typing import List

import jmespath

from chadselect._functions import split_functions, parse_and_apply

logger = logging.getLogger(__name__)


def process(jmespath_with_functions: str, raw_json: str) -> List[str]:
    """Run a JMESPath expression against JSON content.

    Returns all result values stringified. Supports ``>>`` function piping.
    """
    expr, func_chain = split_functions(jmespath_with_functions)

    try:
        data = json.loads(raw_json)
    except json.JSONDecodeError as e:
        logger.warning("Invalid JSON content: %s", e)
        return []

    try:
        result = jmespath.search(expr, data)
    except Exception as e:
        logger.warning("JMESPath failed for '%s': %s", expr, e)
        return []

    results = _to_string_list(result)

    if func_chain.strip():
        results = parse_and_apply(results, func_chain)

    return results


def _to_string_list(value) -> List[str]:
    """Convert a JMESPath result into a flat list of strings."""
    if value is None:
        return []
    if isinstance(value, list):
        out: List[str] = []
        for item in value:
            s = _stringify(item)
            if s:
                out.append(s)
        return out
    s = _stringify(value)
    return [s] if s else []


def _stringify(value) -> str:
    """Convert a single value to string."""
    if value is None:
        return ""
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (dict, list)):
        return json.dumps(value, separators=(",", ":"))
    return str(value)
