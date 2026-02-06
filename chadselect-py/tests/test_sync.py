"""Sync tests for the ChadSelect Python bindings."""

from chadselect import ChadSelect


HTML = """
<div id="inventory">
  <div class="vehicle">
    <h2 class="title">2024 Accord</h2>
    <span class="price">$32,000</span>
    <span class="vin">VIN: 1HGCM82633A123456</span>
    <div class="details">
      <div class="item"><span class="label">Mileage:</span> <span class="value">15,000 mi</span></div>
      <div class="item"><span class="label">Color:</span> <span class="value">Black Metallic</span></div>
    </div>
    <a class="link" href="/vehicle/1001">View Details</a>
  </div>
  <div class="vehicle">
    <h2 class="title">2024 Civic</h2>
    <span class="price">$28,500</span>
    <span class="vin">VIN: 2HGCM82633B654321</span>
  </div>
</div>
"""

JSON = '{"store": {"inventory": [{"name": "Widget", "price": 25}, {"name": "Gadget", "price": 50}]}}'


# ── Basic lifecycle ──────────────────────────────────────────────────────────


def test_new_instance():
    cs = ChadSelect()
    assert cs.content_count() == 0
    assert len(cs) == 0


def test_add_and_clear():
    cs = ChadSelect()
    cs.add_html(HTML)
    cs.add_json(JSON)
    cs.add_text("hello world")
    assert cs.content_count() == 3
    cs.clear()
    assert cs.content_count() == 0


def test_repr():
    cs = ChadSelect()
    cs.add_html(HTML)
    assert "content_count=1" in repr(cs)


# ── CSS ──────────────────────────────────────────────────────────────────────


def test_css_select():
    cs = ChadSelect()
    cs.add_html(HTML)
    assert cs.select(0, "css:.price") == "$32,000"


def test_css_query_all():
    cs = ChadSelect()
    cs.add_html(HTML)
    prices = cs.query(-1, "css:.price")
    assert prices == ["$32,000", "$28,500"]


def test_css_with_functions():
    cs = ChadSelect()
    cs.add_html(HTML)
    result = cs.select(0, "css:.vin >> substring-after('VIN: ')")
    assert result == "1HGCM82633A123456"


def test_css_get_attr():
    cs = ChadSelect()
    cs.add_html(HTML)
    href = cs.select(0, "css:a.link >> get-attr('href')")
    assert href == "/vehicle/1001"


# ── XPath ────────────────────────────────────────────────────────────────────


def test_xpath_select():
    cs = ChadSelect()
    cs.add_html(HTML)
    title = cs.select(0, "xpath://h2[@class='title']/text()")
    assert title == "2024 Accord"


def test_xpath_query_all():
    cs = ChadSelect()
    cs.add_html(HTML)
    titles = cs.query(-1, "xpath://h2[@class='title']/text()")
    assert titles == ["2024 Accord", "2024 Civic"]


# ── Regex ────────────────────────────────────────────────────────────────────


def test_regex_select():
    cs = ChadSelect()
    cs.add_html(HTML)
    vin = cs.select(0, r"regex:VIN:\s*([\w]+)")
    assert vin == "1HGCM82633A123456"


def test_regex_no_prefix_defaults():
    cs = ChadSelect()
    cs.add_text("price: $42")
    assert cs.select(0, r"\$(\d+)") == "42"


def test_regex_no_match():
    cs = ChadSelect()
    cs.add_text("hello")
    assert cs.select(0, r"regex:(\d+)") == ""


# ── JMESPath ─────────────────────────────────────────────────────────────────


def test_json_select():
    cs = ChadSelect()
    cs.add_json(JSON)
    assert cs.select(0, "json:store.inventory[0].name") == "Widget"


def test_json_query_all():
    cs = ChadSelect()
    cs.add_json(JSON)
    names = cs.query(-1, "json:store.inventory[].name")
    assert names == ["Widget", "Gadget"]


# ── select_first ─────────────────────────────────────────────────────────────


def test_select_first_fallback():
    cs = ChadSelect()
    cs.add_html(HTML)
    result = cs.select_first([
        (0, "css:#nonexistent"),
        (0, "css:.price"),
    ])
    assert result == ["$32,000"]


def test_select_first_all_miss():
    cs = ChadSelect()
    cs.add_html(HTML)
    result = cs.select_first([
        (0, "css:#nope"),
        (0, "xpath://div[@id='nope']/text()"),
    ])
    assert result == []


# ── select_many ──────────────────────────────────────────────────────────────


def test_select_many_combines():
    cs = ChadSelect()
    cs.add_html(HTML)
    results = cs.select_many([
        (-1, "css:.price"),
        (-1, "css:.title"),
    ])
    assert "$32,000" in results
    assert "2024 Accord" in results


# ── Multi-content ────────────────────────────────────────────────────────────


def test_regex_spans_all_content():
    cs = ChadSelect()
    cs.add_text("id=100")
    cs.add_text("id=200")
    cs.add_html("<span>id=300</span>")
    results = cs.query(-1, r"regex:id=(\d+)")
    assert set(results) == {"100", "200", "300"}
