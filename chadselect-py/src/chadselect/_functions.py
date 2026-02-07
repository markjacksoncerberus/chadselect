"""
Post-processing text functions — shared by all engines.

Functions are chained using the ``>>`` delimiter after a selector expression::

    css:.price >> normalize-space() >> uppercase()
    xpath://div/text() >> substring-after('VIN: ') >> substring(0, 3)

Mirrors the Rust crate's ``functions.rs`` exactly.
"""

from __future__ import annotations

import re
from typing import List, Tuple

from chadselect._query import FUNCTION_PIPE


def supported_text_functions() -> List[str]:
    """Return the list of all supported text function signatures."""
    return [
        "normalize-space()",
        "trim()",
        "uppercase()",
        "lowercase()",
        "substring(start, length)",
        "substring-after('delimiter')",
        "substring-before('delimiter')",
        "replace('find', 'replace')",
        "get-attr('attribute')",
    ]


def split_functions(input_str: str) -> Tuple[str, str]:
    """Split ``expression >> func1() >> func2()`` into ``(expression, func_chain_str)``.

    Returns ``(expression, "")`` if no ``>>`` pipe is present.
    """
    pos = input_str.find(FUNCTION_PIPE)
    if pos == -1:
        return input_str.strip(), ""
    return input_str[:pos].strip(), input_str[pos + len(FUNCTION_PIPE):]


def parse_and_apply(results: List[str], func_chain_str: str) -> List[str]:
    """Parse a function chain string and apply it to results."""
    if not func_chain_str.strip():
        return results

    for func_str in func_chain_str.split(FUNCTION_PIPE):
        func_str = func_str.strip()
        if not func_str:
            continue
        results = _apply_one(results, func_str)
        # Filter empty results after each step (matches Rust behavior)
        results = [r for r in results if r]
    return results


def _apply_one(results: List[str], func_str: str) -> List[str]:
    """Apply a single function to all results."""
    paren = func_str.find("(")
    if paren == -1:
        # Shorthand without parens — e.g. "trim"
        name = func_str.strip()
        args_str = ""
    else:
        name = func_str[:paren].strip()
        end = func_str.rfind(")")
        args_str = func_str[paren + 1: end if end != -1 else len(func_str)]

    if name == "normalize-space":
        return [re.sub(r"\s+", " ", s).strip() for s in results]

    if name == "trim":
        return [s.strip() for s in results]

    if name == "uppercase":
        return [s.upper() for s in results]

    if name == "lowercase":
        return [s.lower() for s in results]

    if name == "substring":
        args = [a.strip() for a in args_str.split(",")]
        if len(args) >= 2:
            try:
                start, length = int(args[0]), int(args[1])
                return [s[start: start + length] for s in results]
            except ValueError:
                return results
        return results

    if name == "substring-after":
        delim = args_str.strip().strip("\"'")
        out = []
        for s in results:
            idx = s.find(delim)
            out.append(s[idx + len(delim):] if idx != -1 else "")
        # Filter out empty results (matches Rust behavior)
        return [r for r in out if r]

    if name == "substring-before":
        delim = args_str.strip().strip("\"'")
        out = []
        for s in results:
            idx = s.find(delim)
            out.append(s[:idx] if idx != -1 else s)
        return out

    if name == "replace":
        args = _parse_two_string_args(args_str)
        if args:
            find, repl = args
            return [s.replace(find, repl) for s in results]
        return results

    if name == "get-attr":
        # Handled specially by the CSS engine — pass through here
        # (the attr name is extracted at the engine level)
        return results

    # Unknown function — skip silently
    return results


def _parse_two_string_args(args_str: str) -> Tuple[str, str] | None:
    """Parse ``'find', 'replace'`` from an argument string."""
    # Match 'x', 'y' or "x", "y"
    m = re.match(r"""['"](.*?)['"],\s*['"](.*?)['"]""", args_str.strip())
    if m:
        return m.group(1), m.group(2)
    return None
