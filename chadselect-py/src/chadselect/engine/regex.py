"""Regex engine — powered by the ``re`` stdlib module."""

from __future__ import annotations

import logging
import re
from typing import List

from chadselect._functions import split_functions, parse_and_apply

logger = logging.getLogger(__name__)


def process(pattern_with_functions: str, content: str) -> List[str]:
    """Run a regex against content, returning capture groups or full matches.

    - If the pattern has capture groups, returns group values.
    - Otherwise, returns full match strings.

    Supports ``>>`` function piping.
    """
    pattern_str, func_chain = split_functions(pattern_with_functions)

    try:
        compiled = re.compile(pattern_str)
    except re.error as e:
        logger.warning("Invalid regex '%s': %s", pattern_str, e)
        return []

    results: List[str] = []

    if compiled.groups == 0:
        # No capture groups — return full matches
        results = compiled.findall(content)
    else:
        # Has capture groups
        for match in compiled.finditer(content):
            groups = match.groups()
            for g in groups:
                if g is not None:
                    results.append(g)

    # Filter empty/whitespace-only
    results = [r.strip() for r in results if r.strip()]

    if func_chain.strip():
        results = parse_and_apply(results, func_chain)

    return results
