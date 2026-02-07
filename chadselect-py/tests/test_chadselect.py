"""
Comprehensive tests for ChadSelect — mirrors the Rust crate's full test suite.

Organized by engine (CSS, XPath, Regex, JMESPath), then text functions,
integration tests, and edge cases.
"""

import pytest
from chadselect import ChadSelect


# ═══════════════════════════════════════════════════════════════════════════════
#  Shared fixtures
# ═══════════════════════════════════════════════════════════════════════════════

HTML = """\
<div class="test">
    <span class="price">  $100  </span>
    <span class="price">$200</span>
    <div class="vin">VIN: 1HGCM82633A123456</div>
    <div class="info">Price: $300</div>
    <a class="link" href="https://example.com">Click here</a>
</div>
"""

PSEUDO_HTML = """\
<div class="container">
    <div class="item">
        <div class="label">Exterior:</div>
        <div class="value">Blue Metallic</div>
    </div>
    <div class="item">
        <div class="label">Interior:</div>
        <div class="value">Black Leather</div>
    </div>
    <div class="item">
        <div class="label">Engine:</div>
        <div class="value">V6 Turbo</div>
    </div>
    <div class="other">
        <span>Not what we want</span>
    </div>
</div>
"""

XPATH_HTML = """\
<html>
    <body>
        <div class="container">
            <h1>Test Title</h1>
            <p>First paragraph</p>
            <p>Second paragraph</p>
            <span id="vin">1GCPAAEK7TZ152448</span>
            <span id="stock">TZ152448</span>
        </div>
    </body>
</html>
"""

JSON_STORE = """\
{
    "store": {
        "name": "Widget World",
        "inventory": [
            {"name": "Widget", "price": 25, "in_stock": true},
            {"name": "Gadget", "price": 50, "in_stock": false},
            {"name": "Doohickey", "price": 10, "in_stock": true}
        ]
    }
}
"""

JSON_SIMPLE = '{"items": [{"name": "Alpha", "price": 10}, {"name": "Beta", "price": 20}]}'


# ═══════════════════════════════════════════════════════════════════════════════
#  Content Management
# ═══════════════════════════════════════════════════════════════════════════════

class TestContentManagement:
    def test_content_count(self):
        cs = ChadSelect()
        assert cs.content_count() == 0
        cs.add_text("a")
        cs.add_html("<b>b</b>")
        cs.add_json('{"c": 3}')
        assert cs.content_count() == 3

    def test_clear(self):
        cs = ChadSelect()
        cs.add_text("a")
        cs.add_text("b")
        cs.clear()
        assert cs.content_count() == 0
        assert cs.query(-1, "regex:.") == []

    def test_repr_and_len(self):
        cs = ChadSelect()
        assert len(cs) == 0
        cs.add_html(HTML)
        assert len(cs) == 1
        assert "content_count=1" in repr(cs)

    def test_query_on_empty_returns_empty(self):
        cs = ChadSelect()
        assert cs.query(-1, "regex:anything") == []


# ═══════════════════════════════════════════════════════════════════════════════
#  CSS Engine
# ═══════════════════════════════════════════════════════════════════════════════

class TestCSS:
    def setup_method(self):
        self.cs = ChadSelect()
        self.cs.add_html(HTML)

    def test_select_by_class(self):
        results = self.cs.query(-1, "css:.price")
        assert len(results) == 2

    def test_select_by_class_first(self):
        result = self.cs.select(0, "css:.price")
        assert result != ""

    def test_select_by_tag(self):
        results = self.cs.query(-1, "css:a")
        assert results == ["Click here"]

    def test_normalize_space(self):
        results = self.cs.query(-1, "css:.price >> normalize-space()")
        assert results == ["$100", "$200"]

    def test_replace_function(self):
        results = self.cs.query(-1, 'css:.price >> normalize-space() >> replace("$", "USD ")')
        assert results == ["USD 100", "USD 200"]

    def test_substring_after(self):
        results = self.cs.query(-1, "css:.vin >> substring-after('VIN: ')")
        assert results == ["1HGCM82633A123456"]

    def test_substring_before(self):
        results = self.cs.query(-1, "css:.info >> substring-before(': ')")
        assert results == ["Price"]

    def test_chained_functions(self):
        results = self.cs.query(-1, "css:.vin >> substring-after('VIN: ') >> substring(0, 3) >> lowercase()")
        assert results == ["1hg"]

    def test_get_attr(self):
        results = self.cs.query(-1, "css:a.link >> get-attr('href')")
        assert results == ["https://example.com"]

    def test_get_attr_missing(self):
        results = self.cs.query(-1, "css:a.link >> get-attr('data-nope')")
        assert results == []

    def test_index_first(self):
        results = self.cs.query(0, "css:.price")
        assert len(results) == 1

    def test_index_out_of_bounds(self):
        results = self.cs.query(10, "css:.price")
        assert results == []

    def test_no_match(self):
        results = self.cs.query(-1, "css:.nonexistent")
        assert results == []

    def test_invalid_selector(self):
        results = self.cs.query(-1, "css:>>>invalid<<<")
        assert results == []

    def test_css_only_hits_html(self):
        cs = ChadSelect()
        cs.add_text("not html")
        cs.add_json('{"x": 1}')
        cs.add_html("<span class='x'>found</span>")
        results = cs.query(-1, "css:.x")
        assert results == ["found"]

    def test_get_attr_data_attribute(self):
        cs = ChadSelect()
        cs.add_html('<div class="product" data-sku="SK-100">Product</div>')
        result = cs.query(-1, "css:div.product >> get-attr('data-sku')")
        assert result == ["SK-100"]


# ═══════════════════════════════════════════════════════════════════════════════
#  CSS Text Pseudo-selectors
# ═══════════════════════════════════════════════════════════════════════════════

class TestCSSPseudo:
    def setup_method(self):
        self.cs = ChadSelect()
        self.cs.add_html(PSEUDO_HTML)

    def test_has_text(self):
        results = self.cs.query(-1, "css:.item:has-text('Exterior:') .value")
        assert results == ["Blue Metallic"]

    def test_contains_text(self):
        results = self.cs.query(-1, "css:.label:contains-text('Interior:')")
        assert results == ["Interior:"]

    def test_text_equals(self):
        results = self.cs.query(-1, "css:.value:text-equals('V6 Turbo')")
        assert results == ["V6 Turbo"]

    def test_text_starts(self):
        results = self.cs.query(-1, "css:.value:text-starts('Black')")
        assert results == ["Black Leather"]

    def test_text_ends(self):
        results = self.cs.query(-1, "css:.value:text-ends('Metallic')")
        assert results == ["Blue Metallic"]

    def test_pseudo_with_function(self):
        results = self.cs.query(-1, "css:.item:has-text('Engine:') .value >> uppercase()")
        assert results == ["V6 TURBO"]

    def test_pseudo_with_trim(self):
        results = self.cs.query(-1, "css:.item:has-text('Interior') .value >> trim()")
        assert results == ["Black Leather"]


# ═══════════════════════════════════════════════════════════════════════════════
#  XPath Engine
# ═══════════════════════════════════════════════════════════════════════════════

class TestXPath:
    def setup_method(self):
        self.cs = ChadSelect()
        self.cs.add_html(XPATH_HTML)

    def test_text_by_tag(self):
        result = self.cs.select(0, "xpath://h1/text()")
        assert result == "Test Title"

    def test_text_by_id(self):
        result = self.cs.select(0, "xpath://span[@id='vin']/text()")
        assert result == "1GCPAAEK7TZ152448"

    def test_multiple_paragraphs(self):
        results = self.cs.query(-1, "xpath://p/text()")
        assert results == ["First paragraph", "Second paragraph"]

    def test_normalize_space_function(self):
        result = self.cs.select(0, "xpath:normalize-space(//h1)")
        assert result == "Test Title"

    def test_string_function(self):
        result = self.cs.select(0, "xpath:string(//span[@id='vin'])")
        assert result == "1GCPAAEK7TZ152448"

    def test_with_normalize_space_postprocess(self):
        cs = ChadSelect()
        cs.add_html('<span class="price">  $100  </span>')
        results = cs.query(-1, "xpath://span[@class='price']/text() >> normalize-space()")
        assert results == ["$100"]

    def test_with_substring_after(self):
        cs = ChadSelect()
        cs.add_html('<div class="vin">VIN: 1HGCM82633A123456</div>')
        results = cs.query(-1, "xpath://div[@class='vin']/text() >> substring-after('VIN: ')")
        assert results == ["1HGCM82633A123456"]

    def test_with_substring_before(self):
        cs = ChadSelect()
        cs.add_html('<div class="info">Price: $300</div>')
        results = cs.query(-1, "xpath://div[@class='info']/text() >> substring-before(': ')")
        assert results == ["Price"]

    def test_chained_functions(self):
        cs = ChadSelect()
        cs.add_html('<div class="vin">VIN: 1HGCM82633A123456</div>')
        results = cs.query(-1, "xpath://div[@class='vin']/text() >> substring-after('VIN: ') >> substring(0, 3) >> uppercase()")
        assert results == ["1HG"]

    def test_normalize_space_vs_trim(self):
        cs = ChadSelect()
        cs.add_html('<p class="description">  This is a great   vehicle!  </p>')
        normalized = cs.query(-1, "xpath://p[@class='description']/text() >> normalize-space()")
        assert normalized == ["This is a great vehicle!"]
        trimmed = cs.query(-1, "xpath://p[@class='description']/text() >> trim()")
        assert trimmed == ["This is a great   vehicle!"]

    def test_invalid_xpath(self):
        results = self.cs.query(-1, "xpath:[[[invalid")
        assert results == []

    def test_no_match(self):
        results = self.cs.query(-1, "xpath://nonexistent/text()")
        assert results == []

    def test_union_operator(self):
        results = self.cs.query(-1, "xpath://span[@id='vin']/text() | //span[@id='stock']/text()")
        assert len(results) == 2
        assert "1GCPAAEK7TZ152448" in results
        assert "TZ152448" in results

    def test_index_selection(self):
        first = self.cs.query(0, "xpath://p/text()")
        assert first == ["First paragraph"]
        second = self.cs.query(1, "xpath://p/text()")
        assert second == ["Second paragraph"]

    def test_attribute_extraction(self):
        cs = ChadSelect()
        cs.add_html('<a class="link" href="/details/42">View</a>')
        result = cs.query(-1, "xpath://a[@class='link']/@href")
        assert result == ["/details/42"]

    def test_xpath_only_hits_html_and_text(self):
        cs = ChadSelect()
        cs.add_json('{"x": 1}')
        assert cs.query(-1, "xpath://span/text()") == []


# ═══════════════════════════════════════════════════════════════════════════════
#  Regex Engine
# ═══════════════════════════════════════════════════════════════════════════════

class TestRegex:
    def test_capture_group(self):
        cs = ChadSelect()
        cs.add_text('vehicleLat":"40.7128"')
        results = cs.query(-1, r'regex:vehicleLat":"([0-9.]+)"')
        assert results == ["40.7128"]

    def test_no_capture_group_full_match(self):
        cs = ChadSelect()
        cs.add_text("price: $100, price: $200")
        results = cs.query(-1, r"regex:\$\d+")
        assert results == ["$100", "$200"]

    def test_multiple_capture_groups(self):
        cs = ChadSelect()
        cs.add_text("2024-01-15")
        results = cs.query(-1, r"regex:(\d{4})-(\d{2})-(\d{2})")
        assert results == ["2024", "01", "15"]

    def test_index_all(self):
        cs = ChadSelect()
        cs.add_text("price: $100, price: $200, price: $300")
        results = cs.query(-1, r"regex:\$(\d+)")
        assert results == ["100", "200", "300"]

    def test_index_zero(self):
        cs = ChadSelect()
        cs.add_text("price: $100, price: $200, price: $300")
        results = cs.query(0, r"regex:\$(\d+)")
        assert results == ["100"]

    def test_index_one(self):
        cs = ChadSelect()
        cs.add_text("price: $100, price: $200, price: $300")
        results = cs.query(1, r"regex:\$(\d+)")
        assert results == ["200"]

    def test_out_of_bounds(self):
        cs = ChadSelect()
        cs.add_text("price: $100")
        results = cs.query(5, r"regex:\$(\d+)")
        assert results == []

    def test_invalid_regex(self):
        cs = ChadSelect()
        cs.add_text("test content")
        results = cs.query(-1, r"regex:[")
        assert results == []

    def test_no_match(self):
        cs = ChadSelect()
        cs.add_text("hello world")
        results = cs.query(-1, r"regex:(\d+)")
        assert results == []

    def test_no_prefix_defaults_to_regex(self):
        cs = ChadSelect()
        cs.add_text("hello world")
        results = cs.query(-1, r"(world)")
        assert results == ["world"]

    def test_select_returns_single(self):
        cs = ChadSelect()
        cs.add_text("lat: 40.7128")
        result = cs.select(0, r"regex:lat:\s*([0-9.]+)")
        assert result == "40.7128"

    def test_select_no_match_returns_empty(self):
        cs = ChadSelect()
        cs.add_text("nothing here")
        result = cs.select(0, r"regex:(\d+)")
        assert result == ""

    def test_across_multiple_content(self):
        cs = ChadSelect()
        cs.add_text("price: $100")
        cs.add_text("price: $200")
        cs.add_html("<span>price: $300</span>")
        results = cs.query(-1, r"regex:\$(\d+)")
        assert results == ["100", "200", "300"]

    def test_works_on_json(self):
        cs = ChadSelect()
        cs.add_json('{"price": 42}')
        results = cs.query(-1, r'regex:"price":\s*(\d+)')
        assert results == ["42"]


# ═══════════════════════════════════════════════════════════════════════════════
#  JMESPath Engine
# ═══════════════════════════════════════════════════════════════════════════════

class TestJMESPath:
    def setup_method(self):
        self.cs = ChadSelect()
        self.cs.add_json(JSON_STORE)

    def test_simple_string_path(self):
        result = self.cs.select(0, "json:store.name")
        assert result == "Widget World"

    def test_nested_array_index(self):
        result = self.cs.select(0, "json:store.inventory[0].name")
        assert result == "Widget"

    def test_number_value(self):
        result = self.cs.select(0, "json:store.inventory[0].price")
        assert result == "25"

    def test_boolean_value(self):
        result = self.cs.select(0, "json:store.inventory[1].in_stock")
        assert result == "false"

    def test_array_projection_names(self):
        results = self.cs.query(-1, "json:store.inventory[].name")
        assert results == ["Widget", "Gadget", "Doohickey"]

    def test_array_projection_prices(self):
        results = self.cs.query(-1, "json:store.inventory[].price")
        assert results == ["25", "50", "10"]

    def test_invalid_jmespath(self):
        results = self.cs.query(-1, "json:`invalid")
        assert results == []

    def test_no_match(self):
        result = self.cs.select(0, "json:nonexistent.path")
        assert result == ""

    def test_invalid_json_content(self):
        cs = ChadSelect()
        cs.add_json("not json at all")
        results = cs.query(-1, "json:whatever")
        assert results == []

    def test_index_first(self):
        results = self.cs.query(0, "json:store.inventory[].name")
        assert results == ["Widget"]

    def test_index_last(self):
        results = self.cs.query(2, "json:store.inventory[].name")
        assert results == ["Doohickey"]

    def test_index_out_of_bounds(self):
        results = self.cs.query(10, "json:store.inventory[].name")
        assert results == []

    def test_json_only_hits_json(self):
        cs = ChadSelect()
        cs.add_html("<div>hello</div>")
        cs.add_text("hello")
        assert cs.query(-1, "json:whatever") == []


# ═══════════════════════════════════════════════════════════════════════════════
#  Text Functions (edge cases from Rust functions_tests.rs)
# ═══════════════════════════════════════════════════════════════════════════════

class TestTextFunctions:
    """Test text functions via CSS queries (the function chain is engine-agnostic)."""

    def _apply(self, text: str, func_chain: str) -> str:
        """Helper: apply a function chain to text via ChadSelect."""
        cs = ChadSelect()
        cs.add_html(f'<span class="t">{text}</span>')
        return cs.select(0, f"css:.t >> {func_chain}")

    def _apply_all(self, text: str, func_chain: str) -> list:
        cs = ChadSelect()
        cs.add_html(f'<span class="t">{text}</span>')
        return cs.query(-1, f"css:.t >> {func_chain}")

    def test_normalize_space(self):
        assert self._apply("  Hello   World  ", "normalize-space()") == "Hello World"

    def test_trim(self):
        assert self._apply("  Hello World  ", "trim()") == "Hello World"

    def test_uppercase(self):
        assert self._apply("Hello World", "uppercase()") == "HELLO WORLD"

    def test_lowercase(self):
        assert self._apply("Hello World", "lowercase()") == "hello world"

    def test_substring(self):
        assert self._apply("Hello World", "substring(0, 5)") == "Hello"
        assert self._apply("Hello World", "substring(6, 5)") == "World"

    def test_substring_out_of_bounds(self):
        assert self._apply("Hello", "substring(10, 5)") == ""
        assert self._apply("Hello", "substring(3, 10)") == "lo"

    def test_substring_after(self):
        assert self._apply("VIN: 1HGCM82633A123456", "substring-after('VIN: ')") == "1HGCM82633A123456"
        assert self._apply("Price: $25,000", "substring-after(': $')") == "25,000"

    def test_substring_after_missing_delimiter(self):
        # Rust returns "" when delimiter not found
        assert self._apply("Hello World", "substring-after('XYZ')") == ""

    def test_substring_before(self):
        assert self._apply("Price: $25,000", "substring-before(': ')") == "Price"
        assert self._apply("user@domain.com", "substring-before('@')") == "user"

    def test_substring_before_missing_delimiter(self):
        # Rust returns original string when delimiter not found
        assert self._apply("Hello World", "substring-before('XYZ')") == "Hello World"

    def test_replace(self):
        assert self._apply("$100", 'replace("$", "USD ")') == "USD 100"

    def test_replace_all_occurrences(self):
        assert self._apply("Hello Hello World", 'replace("Hello", "Hi")') == "Hi Hi World"

    def test_replace_no_match(self):
        assert self._apply("Hello World", 'replace("XYZ", "ABC")') == "Hello World"

    def test_chain_substring_after_substring_lowercase(self):
        assert self._apply("VIN: 1HGCM82633A123456", "substring-after('VIN: ') >> substring(0, 3) >> lowercase()") == "1hg"

    def test_chain_normalize_trim_uppercase(self):
        assert self._apply("  Hello   World  ", "normalize-space() >> trim() >> uppercase()") == "HELLO WORLD"

    def test_chain_empty_result_filters_out(self):
        # substring-after with missing delimiter returns "" → gets filtered
        result = self._apply_all("Hello World", "substring-after('MISSING')")
        assert result == []

    # ── Unicode ──────────────────────────────────────────────────────────

    def test_unicode_normalize_space(self):
        assert self._apply("Hello   🌍", "normalize-space()") == "Hello 🌍"

    def test_unicode_substring(self):
        assert self._apply("Hello 🌍 World", "substring(6, 1)") == "🌍"

    def test_unicode_substring_after(self):
        assert self._apply("価格: ¥1000", "substring-after('価格: ')") == "¥1000"

    def test_unicode_uppercase(self):
        assert self._apply("Héllo Wörld", "uppercase()") == "HÉLLO WÖRLD"

    # ── Edge cases ───────────────────────────────────────────────────────

    def test_delimiter_longer_than_input(self):
        assert self._apply("Hi", "substring-after('This is much longer')") == ""

    def test_delimiter_at_boundaries(self):
        assert self._apply("VIN: 123", "substring-after('VIN: ')") == "123"
        assert self._apply("123: END", "substring-before(': END')") == "123"


# ═══════════════════════════════════════════════════════════════════════════════
#  Integration — select_first, select_many, select_where
# ═══════════════════════════════════════════════════════════════════════════════

class TestSelectFirst:
    def test_returns_first_hit(self):
        cs = ChadSelect()
        cs.add_html('<span id="vin">ABC123</span>')
        result = cs.select_first([
            (0, "css:#nonexistent"),
            (0, "xpath://span[@id='vin']/text()"),
            (0, "css:#vin"),
        ])
        assert result == ["ABC123"]

    def test_returns_empty_when_all_miss(self):
        cs = ChadSelect()
        cs.add_text("nothing useful")
        result = cs.select_first([
            (0, "css:.nope"),
            (0, r"regex:(\d+)"),
        ])
        assert result == []


class TestSelectMany:
    def test_combines_unique_results(self):
        cs = ChadSelect()
        cs.add_html("""
            <span class="a">Alpha</span>
            <span class="b">Beta</span>
            <span class="c">Alpha</span>
        """)
        results = cs.select_many([(-1, "css:.a"), (-1, "css:.b"), (-1, "css:.c")])
        assert "Alpha" in results
        assert "Beta" in results
        # Alpha deduped
        assert len(results) == 2


class TestSelectWhere:
    def test_rejects_zero(self):
        cs = ChadSelect()
        cs.add_text("price: 0")
        assert cs.select(0, r"(\d+)") == "0"
        r = cs.select_where(0, r"(\d+)", lambda s: s != "0")
        assert r == ""

    def test_accepts_non_zero(self):
        cs = ChadSelect()
        cs.add_text("price: 42")
        r = cs.select_where(0, r"(\d+)", lambda s: s != "0")
        assert r == "42"

    def test_min_length_validator(self):
        cs = ChadSelect()
        cs.add_html('<span class="v">AB</span>')
        r = cs.select_where(0, "css:.v", lambda s: len(s) >= 3)
        assert r == ""

        cs.clear()
        cs.add_html('<span class="v">ABCDEF</span>')
        r = cs.select_where(0, "css:.v", lambda s: len(s) >= 3)
        assert r == "ABCDEF"

    def test_numeric_range_validator(self):
        cs = ChadSelect()
        cs.add_json('{"price": 5}')
        r = cs.select_where(0, "json:price", lambda s: float(s) > 10.0)
        assert r == ""

        cs.clear()
        cs.add_json('{"price": 49.99}')
        r = cs.select_where(0, "json:price", lambda s: float(s) > 10.0)
        assert r == "49.99"


class TestSelectFirstWhere:
    def test_skips_zero_result(self):
        cs = ChadSelect()
        cs.add_text("a: 0\nb: 99")
        r = cs.select_first_where(
            [(0, r"a: (\d+)"), (0, r"b: (\d+)")],
            lambda s: s != "0",
        )
        assert r == ["99"]

    def test_all_rejected_returns_empty(self):
        cs = ChadSelect()
        cs.add_text("val: 0")
        r = cs.select_first_where(
            [(0, r"(\d+)")],
            lambda s: float(s) > 100.0,
        )
        assert r == []


class TestSelectManyWhere:
    def test_filters_results(self):
        cs = ChadSelect()
        cs.add_text("1 0 42 0 7")
        r = cs.select_many_where(
            [(-1, r"(\d+)")],
            lambda s: s != "0",
        )
        assert "0" not in r
        assert "1" in r
        assert "42" in r
        assert "7" in r


# ═══════════════════════════════════════════════════════════════════════════════
#  query_batch
# ═══════════════════════════════════════════════════════════════════════════════

class TestQueryBatch:
    def test_batch_returns_correct_count(self):
        cs = ChadSelect()
        cs.add_html(HTML)
        cs.add_json(JSON_SIMPLE)

        results = cs.query_batch([
            (-1, "css:.price"),
            (0, "json:items[0].name"),
            (-1, r"regex:\$\d+"),
        ])
        assert len(results) == 3
        assert len(results[0]) == 2  # two prices
        assert results[1] == ["Alpha"]

    def test_batch_empty_queries(self):
        cs = ChadSelect()
        cs.add_html(HTML)
        assert cs.query_batch([]) == []


# ═══════════════════════════════════════════════════════════════════════════════
#  Mixed content routing
# ═══════════════════════════════════════════════════════════════════════════════

class TestMixedContent:
    def test_css_only_on_html(self):
        cs = ChadSelect()
        cs.add_html(HTML)
        cs.add_json(JSON_SIMPLE)
        cs.add_text("plain text")
        result = cs.query(-1, "css:.price")
        assert len(result) == 2

    def test_json_only_on_json(self):
        cs = ChadSelect()
        cs.add_html(HTML)
        cs.add_json(JSON_SIMPLE)
        result = cs.query(-1, "json:items[].name")
        assert result == ["Alpha", "Beta"]

    def test_regex_spans_all(self):
        cs = ChadSelect()
        cs.add_text("id=100")
        cs.add_html("<div>id=200</div>")
        cs.add_json('{"id": "id=300"}')
        results = cs.query(-1, r"regex:id=(\d+)")
        assert results == ["100", "200", "300"]

    def test_json_skips_html(self):
        cs = ChadSelect()
        cs.add_html("<div>hello</div>")
        assert cs.query(-1, "json:whatever") == []

    def test_json_skips_text(self):
        cs = ChadSelect()
        cs.add_text("hello")
        assert cs.query(-1, "json:whatever") == []


# ═══════════════════════════════════════════════════════════════════════════════
#  XPath delimiter safety (| vs >>)
# ═══════════════════════════════════════════════════════════════════════════════

class TestDelimiterSafety:
    def test_xpath_pipe_not_confused_with_functions(self):
        cs = ChadSelect()
        cs.add_html("""
            <span class="a">Alpha</span>
            <span class="b">Beta</span>
        """)
        results = cs.query(-1, "xpath://span[@class='a']/text() | //span[@class='b']/text()")
        assert len(results) == 2
        assert "Alpha" in results
        assert "Beta" in results

    def test_double_arrow_pipe_with_xpath_union(self):
        cs = ChadSelect()
        cs.add_html("""
            <span class="a">  Alpha  </span>
            <span class="b">  Beta  </span>
        """)
        results = cs.query(-1, "xpath://span[@class='a']/text() | //span[@class='b']/text() >> normalize-space()")
        for r in results:
            assert not r.startswith(" ")
            assert not r.endswith(" ")
