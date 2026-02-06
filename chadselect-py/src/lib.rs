//! PyO3 bindings for the ChadSelect library.
//!
//! `ChadSelect` internally uses `RefCell`-based caching (via sxd-document)
//! which makes it `!Send + !Sync`. This is fine for Python because each
//! `ChadSelect` instance lives on one thread. The GIL is held during Rust
//! execution, but our benchmarks show queries complete in 100µs–3ms, so
//! GIL contention is negligible.
//!
//! For async usage, the Python-side `AsyncChadSelect` dispatches calls via
//! `asyncio.to_thread()`, which runs each call on a thread-pool thread.
//! Each `AsyncChadSelect` owns its own `ChadSelect` so there is no sharing.

use std::cell::RefCell;

use pyo3::prelude::*;
use pyo3::types::PyList;

use chadselect::ChadSelect;

/// Rust-powered data extraction — Regex, XPath 1.0, CSS Selectors, and
/// JMESPath behind one query interface.
///
/// Usage::
///
///     from chadselect import ChadSelect
///
///     cs = ChadSelect()
///     cs.add_html('<span class="price">$49.99</span>')
///     price = cs.select(0, "css:.price")
#[pyclass(name = "ChadSelect", unsendable)]
struct PyChadSelect {
    inner: RefCell<ChadSelect>,
}

#[pymethods]
impl PyChadSelect {
    #[new]
    fn new() -> Self {
        Self {
            inner: RefCell::new(ChadSelect::new()),
        }
    }

    // ── Content management ──────────────────────────────────────────────

    /// Add plain text content.
    fn add_text(&self, content: String) {
        self.inner.borrow_mut().add_text(content);
    }

    /// Add HTML content (compatible with CSS, XPath, and Regex).
    fn add_html(&self, content: String) {
        self.inner.borrow_mut().add_html(content);
    }

    /// Add JSON content (compatible with JMESPath and Regex).
    fn add_json(&self, content: String) {
        self.inner.borrow_mut().add_json(content);
    }

    /// Return the number of loaded content items.
    fn content_count(&self) -> usize {
        self.inner.borrow().content_count()
    }

    /// Remove all loaded content.
    fn clear(&self) {
        self.inner.borrow_mut().clear();
    }

    // ── Querying ────────────────────────────────────────────────────────

    /// Query all loaded content and return matching results.
    ///
    /// - ``index = -1`` returns **all** matches.
    /// - ``index >= 0`` returns the match at that position (or empty list).
    fn query<'py>(
        &self,
        py: Python<'py>,
        index: i32,
        query_str: &str,
    ) -> PyResult<Bound<'py, PyList>> {
        let results = self.inner.borrow_mut().query(index, query_str);
        PyList::new(py, &results)
    }

    /// Return a single result string, or an empty string on no match.
    ///
    /// A result is valid when it is non-empty and non-whitespace.
    fn select(&self, index: i32, query_str: &str) -> String {
        self.inner.borrow_mut().select(index, query_str)
    }

    /// Try multiple queries in order and return the first valid result set.
    fn select_first<'py>(
        &self,
        py: Python<'py>,
        queries: Vec<(i32, String)>,
    ) -> PyResult<Bound<'py, PyList>> {
        let refs: Vec<(i32, &str)> =
            queries.iter().map(|(i, s)| (*i, s.as_str())).collect();
        let results = self.inner.borrow_mut().select_first(refs);
        PyList::new(py, &results)
    }

    /// Run multiple queries and return the combined unique results.
    fn select_many<'py>(
        &self,
        py: Python<'py>,
        queries: Vec<(i32, String)>,
    ) -> PyResult<Bound<'py, PyList>> {
        let refs: Vec<(i32, &str)> =
            queries.iter().map(|(i, s)| (*i, s.as_str())).collect();
        let results = self.inner.borrow_mut().select_many(refs);
        PyList::new(py, &results)
    }

    fn __repr__(&self) -> String {
        format!(
            "ChadSelect(content_count={})",
            self.inner.borrow().content_count()
        )
    }

    fn __len__(&self) -> usize {
        self.inner.borrow().content_count()
    }
}

/// The native Rust module — use ``import chadselect`` instead.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyChadSelect>()?;
    Ok(())
}
