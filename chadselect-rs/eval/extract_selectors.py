#!/usr/bin/env python3
"""
Extract every real CSS / XPath (and json/regex, for completeness) selector that
the hermes_rust crawler fleet feeds into chadselect, so the eval harness can
replay them against a giant HTML doc and find the CPU-pegging ones.

Why this is not just `grep`:
  * Selectors are Rust string literals. Many span multiple source lines via the
    `"... \<newline>   ..."` continuation form (backslash-newline + leading
    whitespace is elided by the Rust lexer).
  * Some are raw strings `r#"..."#` / `r"..."` (used when the selector contains
    backslashes, e.g. regex).
  * Most carry a `>> func(...)` post-processing pipe that is NOT part of the
    selector and must be stripped before timing the DOM traversal.

So we lex Rust string literals properly, reproduce the continuation/escape
semantics, keep the ones with a known engine prefix, strip the pipe, and dedup.

Output: eval/selectors.json  — a list of {engine, selector, count, sites}.
"""
import json
import os
import re
import sys
from collections import defaultdict

CRAWLER_DIR = os.path.join(os.path.dirname(__file__), "..", "..", "..", "hermes_rust", "src", "crawlers")
OUT = os.path.join(os.path.dirname(__file__), "selectors.json")

PREFIXES = ("css:", "xpath:", "json:", "regex:")
PIPE = ">>"


def lex_string_literals(src):
    """Yield decoded Rust string-literal contents from `src`.

    Handles: normal "..." with escapes incl. backslash-newline continuation,
    and raw strings r"..." / r#"..."# / r##"..."## etc. Good enough for the
    crawler sources (does not need to be a full Rust lexer)."""
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        # raw string: r, optional #+, "
        if c == "r" and i + 1 < n and (src[i + 1] == '"' or src[i + 1] == "#"):
            j = i + 1
            hashes = 0
            while j < n and src[j] == "#":
                hashes += 1
                j += 1
            if j < n and src[j] == '"':
                terminator = '"' + ("#" * hashes)
                start = j + 1
                end = src.find(terminator, start)
                if end == -1:
                    i = j + 1
                    continue
                yield src[start:end]
                i = end + len(terminator)
                continue
        if c == '"':
            # normal string literal with escapes
            j = i + 1
            buf = []
            while j < n:
                ch = src[j]
                if ch == "\\":
                    nxt = src[j + 1] if j + 1 < n else ""
                    if nxt == "\n":
                        # line continuation: skip backslash, newline, and the
                        # leading whitespace of the next line
                        j += 2
                        while j < n and src[j] in " \t":
                            j += 1
                        continue
                    # ordinary escape — keep the escaped char verbatim. For our
                    # purposes (selectors), \' \" \\ etc. just map to the char.
                    mapping = {"n": "\n", "t": "\t", "r": "\r", "\\": "\\",
                               '"': '"', "'": "'", "0": "\0"}
                    buf.append(mapping.get(nxt, nxt))
                    j += 2
                    continue
                if ch == '"':
                    yield "".join(buf)
                    j += 1
                    break
                buf.append(ch)
                j += 1
            i = j
            continue
        i += 1


def strip_pipe(sel):
    """Drop the `>> func(...) >> ...` post-processing chain."""
    idx = sel.find(PIPE)
    if idx != -1:
        sel = sel[:idx]
    return sel.strip()


def main():
    # engine -> selector -> set(sites)
    seen = defaultdict(lambda: defaultdict(set))
    files = sorted(f for f in os.listdir(CRAWLER_DIR) if f.endswith(".rs"))
    total_literals = 0
    for fname in files:
        site = fname[:-3]
        path = os.path.join(CRAWLER_DIR, fname)
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            src = fh.read()
        for lit in lex_string_literals(src):
            for pfx in PREFIXES:
                if lit.startswith(pfx):
                    engine = pfx[:-1]
                    sel = strip_pipe(lit[len(pfx):])
                    if sel:
                        seen[engine][sel].add(site)
                        total_literals += 1
                    break

    out = []
    for engine in ("css", "xpath", "json", "regex"):
        for sel, sites in sorted(seen[engine].items()):
            out.append({
                "engine": engine,
                "selector": sel,
                "count": len(sites),
                "sites": sorted(sites),
            })

    with open(OUT, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)

    # summary to stderr
    by_engine = defaultdict(int)
    for rec in out:
        by_engine[rec["engine"]] += 1
    print(f"parsed {len(files)} crawler files; {total_literals} prefixed literals", file=sys.stderr)
    print("unique selectors by engine:", file=sys.stderr)
    for e in ("css", "xpath", "json", "regex"):
        print(f"  {e:6} {by_engine[e]}", file=sys.stderr)
    print(f"wrote {OUT} ({len(out)} unique total)", file=sys.stderr)


if __name__ == "__main__":
    main()
