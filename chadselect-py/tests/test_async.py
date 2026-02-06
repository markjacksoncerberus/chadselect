"""Async tests for the AsyncChadSelect wrapper."""

import asyncio
import pytest
from chadselect import AsyncChadSelect


HTML = """
<div>
  <span class="price">$49.99</span>
  <span class="price">$29.99</span>
  <span class="vin">VIN: 1HGCM82633A123456</span>
  <a class="link" href="/details/42">View</a>
</div>
"""

JSON = '{"items": [{"name": "Alpha", "price": 10}, {"name": "Beta", "price": 20}]}'


@pytest.mark.asyncio
async def test_async_select():
    cs = AsyncChadSelect()
    cs.add_html(HTML)
    price = await cs.select(0, "css:.price")
    assert price == "$49.99"


@pytest.mark.asyncio
async def test_async_query_all():
    cs = AsyncChadSelect()
    cs.add_html(HTML)
    prices = await cs.query(-1, "css:.price")
    assert prices == ["$49.99", "$29.99"]


@pytest.mark.asyncio
async def test_async_select_first():
    cs = AsyncChadSelect()
    cs.add_html(HTML)
    result = await cs.select_first([
        (0, "css:#nope"),
        (0, "css:.vin"),
    ])
    assert "VIN: 1HGCM82633A123456" in result[0]


@pytest.mark.asyncio
async def test_async_select_many():
    cs = AsyncChadSelect()
    cs.add_html(HTML)
    results = await cs.select_many([
        (-1, "css:.price"),
        (-1, "css:.vin"),
    ])
    assert "$49.99" in results
    assert "VIN: 1HGCM82633A123456" in results


@pytest.mark.asyncio
async def test_async_json():
    cs = AsyncChadSelect()
    cs.add_json(JSON)
    name = await cs.select(0, "json:items[0].name")
    assert name == "Alpha"


@pytest.mark.asyncio
async def test_async_with_functions():
    cs = AsyncChadSelect()
    cs.add_html(HTML)
    vin = await cs.select(0, "css:.vin >> substring-after('VIN: ') >> uppercase()")
    assert vin == "1HGCM82633A123456"


@pytest.mark.asyncio
async def test_async_repr_and_len():
    cs = AsyncChadSelect()
    assert len(cs) == 0
    cs.add_html(HTML)
    assert len(cs) == 1
    assert "content_count=1" in repr(cs)


@pytest.mark.asyncio
async def test_concurrent_queries():
    """Multiple async queries run concurrently without blocking each other."""
    cs = AsyncChadSelect()
    cs.add_html(HTML)
    cs.add_json(JSON)

    results = await asyncio.gather(
        cs.select(0, "css:.price"),
        cs.select(0, "json:items[0].name"),
        cs.query(-1, r"regex:\$[\d.]+"),
    )

    assert results[0] == "$49.99"
    assert results[1] == "Alpha"
    assert "$49.99" in results[2]
