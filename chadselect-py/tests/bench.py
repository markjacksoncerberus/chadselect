#!/usr/bin/env python3
"""
ChadSelect Python — standalone benchmark.

Uses the **same fixtures and queries** as ``chadselect-rs/benches/engine_bench.rs``
so results are directly comparable.

Usage:
    python tests/bench.py            # default 2000 iterations
    python tests/bench.py -n 500     # custom iteration count
"""

from __future__ import annotations

import argparse
import statistics
import time
from typing import List, Tuple

from chadselect import ChadSelect

# ═══════════════════════════════════════════════════════════════════════════════
#  Fixtures — identical to chadselect-rs/benches/engine_bench.rs
# ═══════════════════════════════════════════════════════════════════════════════

COLORS = ["Black", "White", "Silver", "Red", "Blue"]
ENGINES = ["2.0L Turbo", "3.6L V6", "5.0L V8", "Electric", "Hybrid"]
CAT1 = ["tools", "electronics", "home", "garden", "toys"]
CAT2 = ["home", "office", "outdoor", "kitchen", "garage"]


def _ecommerce_html() -> str:
    """200-product e-commerce page (~30 KB)."""
    rows = []
    for i in range(200):
        rows.append(
            f'  <div class="product" data-sku="SKU-{i:03d}">\n'
            f'    <h2 class="title">Product {i}</h2>\n'
            f'    <span class="price original">${100 + i * 5}.99</span>\n'
            f'    <span class="price current">${80 + i * 4}.99</span>\n'
            f'    <span class="vin">VIN: 1HGCM{i:05d}A{100_000 + i * 7:06d}</span>\n'
            f'    <div class="details">\n'
            f'      <div class="spec"><span class="label">Color:</span> <span class="value">{COLORS[i % 5]}</span></div>\n'
            f'      <div class="spec"><span class="label">Engine:</span> <span class="value">{ENGINES[i % 5]}</span></div>\n'
            f'    </div>\n'
            f'    <a class="buy" href="/buy/{i}">Buy Now</a>\n'
            f'  </div>'
        )
    return "<html><body>\n<div class=\"products\">\n" + "\n".join(rows) + "\n</div>\n</body></html>"


def _news_html() -> str:
    """News page with 20 tickers."""
    tickers = [
        ("AAPL", "+2.3"), ("GOOGL", "+1.8"), ("MSFT", "+3.1"), ("AMZN", "-0.4"),
        ("META", "+1.2"), ("NVDA", "+4.5"), ("TSLA", "-1.7"), ("JPM", "+0.9"),
        ("BAC", "+0.3"), ("WMT", "-0.2"), ("DIS", "+1.1"), ("NFLX", "+2.8"),
        ("AMD", "+3.4"), ("INTC", "-0.8"), ("PYPL", "+0.6"), ("SQ", "+1.9"),
        ("UBER", "+0.4"), ("LYFT", "-1.3"), ("SNAP", "+0.7"), ("PINS", "-0.5"),
    ]
    ticker_html = "\n".join(
        f'    <span class="ticker">{sym} {pct}%</span>' for sym, pct in tickers
    )
    return (
        '<html><body>\n'
        '<article class="story">\n'
        '  <h1 class="headline">Markets Rally on Fed Decision</h1>\n'
        '  <div class="byline">By Jane Smith | 2025-12-15</div>\n'
        '  <p class="summary">Major indices climbed as the Federal Reserve held rates steady.</p>\n'
        '  <div class="tickers">\n'
        f'{ticker_html}\n'
        '  </div>\n'
        '</article>\n'
        '</body></html>'
    )


def _api_json() -> str:
    """200-item JSON API response."""
    items = []
    for i in range(200):
        stock = "false" if i % 3 == 0 else "true"
        items.append(
            f'{{"id":{i},"name":"Product {i}","price":{10 + i * 3}.99,'
            f'"in_stock":{stock},'
            f'"categories":["{CAT1[i % 5]}","{CAT2[i % 5]}"],'
            f'"specs":{{"weight":"{1 + i % 10}.{i % 10}kg","color":"{COLORS[i % 5]}"}}}}'
        )
    return '{"api_version":"2.1","products":[' + ",".join(items) + "]}"


# Build once at import time
ECOMMERCE_HTML = _ecommerce_html()
NEWS_HTML = _news_html()
API_JSON = _api_json()

# ═══════════════════════════════════════════════════════════════════════════════
#  Benchmark tasks — (label, content_type, content, index, query)
#  Same 20 queries as the Rust Criterion bench.
# ═══════════════════════════════════════════════════════════════════════════════

TASKS: List[Tuple[str, str, str, int, str]] = [
    # ── CSS (6) ──────────────────────────────────────────────────────────
    ("CSS  — product titles",           "html", ECOMMERCE_HTML, -1, "css:.title"),
    ("CSS  — current prices",           "html", ECOMMERCE_HTML, -1, "css:.price.current"),
    ("CSS  — buy link hrefs",           "html", ECOMMERCE_HTML, -1, "css:a.buy >> get-attr('href')"),
    ("CSS  — ticker symbols",           "html", NEWS_HTML,      -1, "css:.ticker"),
    ("CSS  — first VIN + chain",        "html", ECOMMERCE_HTML,  0, "css:.vin >> substring-after('VIN: ') >> uppercase()"),
    ("CSS  — all data-sku attrs",       "html", ECOMMERCE_HTML, -1, "css:.product >> get-attr('data-sku')"),

    # ── XPath (6) ────────────────────────────────────────────────────────
    ("XPath — current prices",          "html", ECOMMERCE_HTML, -1, "xpath://span[@class='price current']/text()"),
    ("XPath — buy link hrefs",          "html", ECOMMERCE_HTML, -1, "xpath://a[@class='buy']/@href"),
    ("XPath — headline",                "html", NEWS_HTML,       0, "xpath://h1[@class='headline']/text()"),
    ("XPath — all data-skus",           "html", ECOMMERCE_HTML, -1, "xpath://div[@class='product']/@data-sku"),
    ("XPath — union (h1|h2)",           "html", ECOMMERCE_HTML, -1, "xpath://h1/text() | //h2/text()"),
    ("XPath — normalize-space",         "html", NEWS_HTML,       0, "xpath:normalize-space(//h1)"),

    # ── Regex (4) ────────────────────────────────────────────────────────
    ("Regex — dollar prices",           "html", ECOMMERCE_HTML, -1, r"regex:\$[\d,]+\.\d{2}"),
    ("Regex — VIN numbers",             "html", ECOMMERCE_HTML, -1, r"regex:VIN:\s*([\w]+)"),
    ("Regex — data-sku capture",        "html", ECOMMERCE_HTML, -1, r'regex:data-sku="(SKU-\d+)"'),
    ("Regex — pct changes",             "html", NEWS_HTML,      -1, r"regex:[+-]\d+\.\d+%"),

    # ── JMESPath (4) ─────────────────────────────────────────────────────
    ("JSON  — product names",           "json", API_JSON,       -1, "json:products[].name"),
    ("JSON  — in-stock filter",         "json", API_JSON,       -1, "json:products[?in_stock].name"),
    ("JSON  — nested spec",             "json", API_JSON,        0, "json:products[0].specs.weight"),
    ("JSON  — flatten categories",      "json", API_JSON,       -1, "json:products[].categories[]"),
]


# ═══════════════════════════════════════════════════════════════════════════════
#  Runner
# ═══════════════════════════════════════════════════════════════════════════════

def bench_task(
    label: str,
    content_type: str,
    content: str,
    index: int,
    query: str,
    n: int,
) -> dict:
    """Run a single benchmark task *n* times, return timing stats."""
    # Pre-load ChadSelect once (parsing cost is separate from query cost)
    cs = ChadSelect()
    if content_type == "html":
        cs.add_html(content)
    elif content_type == "json":
        cs.add_json(content)
    else:
        cs.add_text(content)

    # Warm-up
    result = cs.query(index, query)

    timings: List[float] = []
    for _ in range(n):
        t0 = time.perf_counter_ns()
        cs.query(index, query)
        t1 = time.perf_counter_ns()
        timings.append((t1 - t0) / 1_000)  # → µs

    timings.sort()
    return {
        "label": label,
        "result_count": len(result),
        "median": statistics.median(timings),
        "mean": statistics.mean(timings),
        "min": min(timings),
        "p95": timings[int(len(timings) * 0.95)],
        "p99": timings[int(len(timings) * 0.99)],
    }


def fmt_us(v: float) -> str:
    if v >= 1000:
        return f"{v / 1000:,.1f}ms"
    return f"{v:,.1f}µs"


def main():
    parser = argparse.ArgumentParser(description="ChadSelect Python benchmark")
    parser.add_argument("-n", type=int, default=2000, help="iterations per task (default: 2000)")
    args = parser.parse_args()
    n = args.n

    print()
    print("═" * 90)
    print(f"  ChadSelect Python Benchmark — {n:,} iterations per task")
    print("═" * 90)

    current_engine = ""
    results = []

    for label, ctype, content, idx, query in TASKS:
        engine = label.split("—")[0].strip()
        if engine != current_engine:
            current_engine = engine
            print()
            print("─" * 90)
            print(f"  {engine}")
            print("─" * 90)
            print(f"  {'Task':<42s} {'Median':>10s} {'Mean':>10s} {'Min':>10s} {'p95':>10s} {'p99':>10s}  Hits")
            print(f"  {'─'*42} {'─'*10} {'─'*10} {'─'*10} {'─'*10} {'─'*10} ─────")

        stats = bench_task(label, ctype, content, idx, query, n)
        results.append(stats)

        short = label.split("— ")[1] if "— " in label else label
        print(
            f"  {short:<42s} "
            f"{fmt_us(stats['median']):>10s} "
            f"{fmt_us(stats['mean']):>10s} "
            f"{fmt_us(stats['min']):>10s} "
            f"{fmt_us(stats['p95']):>10s} "
            f"{fmt_us(stats['p99']):>10s} "
            f" {stats['result_count']:>4d}"
        )

    # Summary
    print()
    print("═" * 90)
    print("  SUMMARY")
    print("═" * 90)
    medians = [r["median"] for r in results]
    print(f"  Tasks run:        {len(results)}")
    print(f"  Fastest median:   {fmt_us(min(medians)):>10s}  ({results[medians.index(min(medians))]['label']})")
    print(f"  Slowest median:   {fmt_us(max(medians)):>10s}  ({results[medians.index(max(medians))]['label']})")
    print(f"  Overall median:   {fmt_us(statistics.median(medians)):>10s}")
    print(f"  Overall mean:     {fmt_us(statistics.mean(medians)):>10s}")
    print("═" * 90)
    print()


if __name__ == "__main__":
    main()
