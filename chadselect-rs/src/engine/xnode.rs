//! XPath-over-`scraper` adapter.
//!
//! This bridges the `scraper`/`html5ever` `ego-tree` DOM (the same tree the CSS
//! engine uses) to `xrust`'s generic [`Node`] trait, so XPath 1.0 can be
//! evaluated over a document **parsed once** by html5ever — no second parse, no
//! conversion to a separate DOM. This replaces the `sxd_html`/`sxd-document`
//! XPath path, whose tree-build was quadratic in memory and time.
//!
//! [`ENode`] is an owned, cheaply-clonable handle: an `Rc<Html>` plus a
//! locator. Attributes are not nodes in ego-tree, so they are represented
//! synthetically (`Loc::Attr`). HTML element/attribute names are reported
//! *without* a namespace, so ordinary `//div`-style queries match (html5ever
//! places HTML elements in the XHTML namespace, which would otherwise break
//! unprefixed name tests).

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use ego_tree::NodeId;
use scraper::node::Node as SNode;
use scraper::Html;

use qualname::{NamespacePrefix, NamespaceUri, NcName, QName};
use xrust::item::{Node, NodeType};
use xrust::output::OutputDefinition;
use xrust::validators::{Schema, ValidationError};
use xrust::value::Value;
use xrust::xdmerror::{Error, ErrorKind};
use xrust::xmldecl::{XMLDecl, XMLDeclBuilder, DTD};

/// A locator into the shared `Html` tree.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Loc {
    /// A real ego-tree node (document, element, text, comment, …).
    Node(NodeId),
    /// A synthetic attribute node: its owning element plus the attribute's
    /// positional index within that element's (deterministically ordered) attrs.
    Attr { owner: NodeId, idx: usize },
}

/// Pre-order rank of every node, so document-order comparisons are O(1).
/// Without this, `cmp_document_order` would re-walk to the root per call and
/// xrust's per-step nodeset sort would become O(n²).
type OrderMap = HashMap<NodeId, u32>;

/// Assign each node a pre-order (document-order) rank in a single pass.
fn build_order(html: &Html) -> OrderMap {
    let mut map = HashMap::new();
    for (rank, n) in html.tree.root().descendants().enumerate() {
        map.insert(n.id(), rank as u32);
    }
    map
}

/// An owned handle to a node in a `scraper`-parsed document. Cheap to clone
/// (two `Rc` bumps + a small locator).
#[derive(Clone)]
pub struct ENode {
    doc: Rc<Html>,
    order: Rc<OrderMap>,
    loc: Loc,
}

impl ENode {
    /// Wrap the document root of an already-parsed `Html` tree, computing the
    /// document-order index once for the whole document.
    pub fn root_of(doc: &Rc<Html>) -> Self {
        ENode {
            doc: doc.clone(),
            order: Rc::new(build_order(doc)),
            loc: Loc::Node(doc.tree.root().id()),
        }
    }

    fn node_at(&self, id: NodeId) -> Self {
        ENode {
            doc: self.doc.clone(),
            order: self.order.clone(),
            loc: Loc::Node(id),
        }
    }

    /// `(rank, attr-slot)` sort key: a node sorts at its pre-order rank; an
    /// attribute sorts immediately after its owning element (slot ≥ 1).
    fn order_key(&self) -> (u32, usize) {
        match self.loc {
            Loc::Node(id) => (*self.order.get(&id).unwrap_or(&u32::MAX), 0),
            Loc::Attr { owner, idx } => (*self.order.get(&owner).unwrap_or(&u32::MAX), idx + 1),
        }
    }

    /// `(name, value)` of the `idx`-th attribute of `owner`, if present.
    fn attr_pair(&self, owner: NodeId, idx: usize) -> Option<(String, String)> {
        match self.doc.tree.get(owner)?.value() {
            SNode::Element(el) => el
                .attrs()
                .nth(idx)
                .map(|(k, v)| (k.to_string(), v.to_string())),
            _ => None,
        }
    }

    /// The XPath string-value: concatenation of all descendant text for
    /// element/document nodes; own content for text/comment/attribute nodes.
    fn string_value(&self) -> String {
        match self.loc {
            Loc::Attr { owner, idx } => {
                self.attr_pair(owner, idx).map(|(_, v)| v).unwrap_or_default()
            }
            Loc::Node(id) => {
                let Some(nref) = self.doc.tree.get(id) else {
                    return String::new();
                };
                match nref.value() {
                    SNode::Text(t) => t.text.to_string(),
                    SNode::Comment(c) => c.comment.to_string(),
                    SNode::Element(_) | SNode::Document | SNode::Fragment => {
                        let mut s = String::new();
                        for d in nref.descendants() {
                            if let SNode::Text(t) = d.value() {
                                s.push_str(&t.text);
                            }
                        }
                        s
                    }
                    _ => String::new(),
                }
            }
        }
    }

    fn collect(v: Vec<ENode>) -> Box<dyn Iterator<Item = ENode>> {
        Box::new(v.into_iter())
    }
}

fn err(msg: &str) -> Error {
    Error::new(ErrorKind::NotImplemented, msg.to_string())
}

thread_local! {
    /// Memoize `QName` construction keyed by local name.
    ///
    /// qualname's string interner takes a global write-lock and linear-scans a
    /// Vec on *every* `NcName::try_from` (~58µs/call, even for a repeated
    /// string). `name()` is called once per node xrust visits, so without this
    /// a single `//x` query over an N-element document costs N lock+scan ops —
    /// seconds for large pages. Element/attribute names are few and repeat
    /// heavily, so memoizing collapses it to O(distinct names).
    static QNAME_MEMO: RefCell<HashMap<String, Option<QName>>> = RefCell::new(HashMap::new());
}

/// Build (or fetch a cached) namespace-free `QName` for an HTML local name.
fn local_qname(local: &str) -> Option<QName> {
    QNAME_MEMO.with(|m| {
        if let Some(q) = m.borrow().get(local) {
            return q.clone();
        }
        let q = NcName::try_from(local).ok().map(QName::from_local_name);
        m.borrow_mut().insert(local.to_string(), q.clone());
        q
    })
}

impl Node for ENode {
    type NodeIterator = Box<dyn Iterator<Item = ENode>>;

    // ── Construction (only new_document is real; used for result documents) ──

    fn new_document() -> Self {
        let html = Html::new_document();
        let root = html.tree.root().id();
        ENode {
            order: Rc::new(build_order(&html)),
            doc: Rc::new(html),
            loc: Loc::Node(root),
        }
    }

    // ── Inspection ──

    fn node_type(&self) -> NodeType {
        match self.loc {
            Loc::Attr { .. } => NodeType::Attribute,
            Loc::Node(id) => match self.doc.tree.get(id).map(|n| n.value()) {
                Some(SNode::Document) | Some(SNode::Fragment) => NodeType::Document,
                Some(SNode::Element(_)) => NodeType::Element,
                Some(SNode::Text(_)) => NodeType::Text,
                Some(SNode::Comment(_)) => NodeType::Comment,
                Some(SNode::ProcessingInstruction(_)) => NodeType::ProcessingInstruction,
                _ => NodeType::Unknown,
            },
        }
    }

    fn is_element(&self) -> bool {
        self.node_type() == NodeType::Element
    }

    fn name(&self) -> Option<QName> {
        let local = match self.loc {
            Loc::Node(id) => match self.doc.tree.get(id)?.value() {
                SNode::Element(el) => el.name().to_string(),
                _ => return None,
            },
            Loc::Attr { owner, idx } => self.attr_pair(owner, idx)?.0,
        };
        local_qname(&local)
    }

    fn to_qname(&self, name: impl AsRef<str>) -> Result<QName, Error> {
        let s = name.as_ref();
        // HTML has no namespaces here; drop any prefix and take the local part.
        let local = s.rsplit(':').next().unwrap_or(s);
        local_qname(local)
            .ok_or_else(|| Error::new(ErrorKind::ParseError, "unable to resolve qualified name"))
    }

    fn to_prefixed_name(&self) -> String {
        self.name()
            .map(|qn| qn.local_name().to_string())
            .unwrap_or_default()
    }

    fn value(&self) -> Rc<Value> {
        let s = match self.loc {
            Loc::Attr { owner, idx } => {
                self.attr_pair(owner, idx).map(|(_, v)| v).unwrap_or_default()
            }
            Loc::Node(id) => match self.doc.tree.get(id).map(|n| n.value()) {
                Some(SNode::Text(t)) => t.text.to_string(),
                Some(SNode::Comment(c)) => c.comment.to_string(),
                _ => String::new(),
            },
        };
        Rc::new(Value::from(s.as_str()))
    }

    fn get_id(&self) -> String {
        match self.loc {
            Loc::Node(id) => format!("{:?}", id),
            Loc::Attr { owner, idx } => format!("{:?}#attr{}", owner, idx),
        }
    }

    fn to_string(&self) -> String {
        self.string_value()
    }

    fn to_xml(&self) -> String {
        self.string_value()
    }

    fn to_xml_with_options(&self, _: &OutputDefinition) -> String {
        self.string_value()
    }

    fn to_json(&self) -> String {
        String::new()
    }

    fn is_same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.doc, &other.doc) && self.loc == other.loc
    }

    fn is_attached(&self) -> bool {
        true
    }

    fn is_unattached(&self) -> bool {
        false
    }

    fn unattached(&self) -> Vec<Self> {
        vec![]
    }

    fn document_order(&self) -> Vec<usize> {
        let (rank, slot) = self.order_key();
        if slot == 0 {
            vec![rank as usize]
        } else {
            vec![rank as usize, slot]
        }
    }

    fn cmp_document_order(&self, other: &Self) -> Ordering {
        self.order_key().cmp(&other.order_key())
    }

    // ── Axes ──

    fn child_iter(&self) -> Self::NodeIterator {
        let mut v = Vec::new();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                if matches!(
                    nref.value(),
                    SNode::Document | SNode::Fragment | SNode::Element(_)
                ) {
                    for c in nref.children() {
                        v.push(self.node_at(c.id()));
                    }
                }
            }
        }
        ENode::collect(v)
    }

    fn ancestor_iter(&self) -> Self::NodeIterator {
        let mut v = Vec::new();
        let start = match self.loc {
            Loc::Node(id) => Some(id),
            Loc::Attr { owner, .. } => {
                v.push(self.node_at(owner));
                Some(owner)
            }
        };
        if let Some(id) = start {
            if let Some(nref) = self.doc.tree.get(id) {
                for a in nref.ancestors() {
                    v.push(self.node_at(a.id()));
                }
            }
        }
        ENode::collect(v)
    }

    fn descend_iter(&self) -> Self::NodeIterator {
        let mut v = Vec::new();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                // descendants() yields self first; skip it for the descendant axis.
                for d in nref.descendants().skip(1) {
                    v.push(self.node_at(d.id()));
                }
            }
        }
        ENode::collect(v)
    }

    fn next_iter(&self) -> Self::NodeIterator {
        let mut v = Vec::new();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                for s in nref.next_siblings() {
                    v.push(self.node_at(s.id()));
                }
            }
        }
        ENode::collect(v)
    }

    fn prev_iter(&self) -> Self::NodeIterator {
        let mut v = Vec::new();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                for s in nref.prev_siblings() {
                    v.push(self.node_at(s.id()));
                }
            }
        }
        ENode::collect(v)
    }

    fn attribute_iter(&self) -> Self::NodeIterator {
        let mut v = Vec::new();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                if let SNode::Element(el) = nref.value() {
                    for idx in 0..el.attrs().count() {
                        v.push(ENode {
                            doc: self.doc.clone(),
                            order: self.order.clone(),
                            loc: Loc::Attr { owner: id, idx },
                        });
                    }
                }
            }
        }
        ENode::collect(v)
    }

    fn namespace_iter(&self) -> Self::NodeIterator {
        ENode::collect(vec![])
    }

    fn get_attribute(&self, a: &QName) -> Rc<Value> {
        let local = a.local_name().to_string();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                if let SNode::Element(el) = nref.value() {
                    if let Some(val) = el.attr(&local) {
                        return Rc::new(Value::from(val));
                    }
                }
            }
        }
        Rc::new(Value::from(""))
    }

    fn get_attribute_node(&self, a: &QName) -> Option<Self> {
        let local = a.local_name().to_string();
        if let Loc::Node(id) = self.loc {
            if let Some(nref) = self.doc.tree.get(id) {
                if let SNode::Element(el) = nref.value() {
                    for (i, (k, _)) in el.attrs().enumerate() {
                        if k == local {
                            return Some(ENode {
                                doc: self.doc.clone(),
                                order: self.order.clone(),
                                loc: Loc::Attr { owner: id, idx: i },
                            });
                        }
                    }
                }
            }
        }
        None
    }

    fn owner_document(&self) -> Self {
        ENode::root_of(&self.doc)
    }

    // ── Namespaces (HTML path is namespace-free) ──

    fn is_in_scope(&self) -> bool {
        false
    }
    fn to_namespace_prefix(&self, _: &NamespaceUri) -> Result<Option<NamespacePrefix>, Error> {
        Err(err("no namespaces"))
    }
    fn to_namespace_uri(&self, _: &Option<NamespacePrefix>) -> Result<NamespaceUri, Error> {
        Err(err("no namespaces"))
    }
    fn as_namespace_prefix(&self) -> Result<Option<&NamespacePrefix>, Error> {
        Err(err("not a namespace node"))
    }
    fn as_namespace_uri(&self) -> Result<&NamespaceUri, Error> {
        Err(err("not a namespace node"))
    }
    fn new_namespace(
        &self,
        _: NamespaceUri,
        _: Option<NamespacePrefix>,
        _: bool,
    ) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn add_namespace(&self, _: Self) -> Result<(), Error> {
        Err(err("read-only"))
    }

    // ── Identity / DTD ──

    fn is_id(&self) -> bool {
        false
    }
    fn is_idrefs(&self) -> bool {
        false
    }
    fn get_dtd(&self) -> Option<DTD> {
        None
    }
    fn set_dtd(&self, _: DTD) -> Result<(), Error> {
        Err(err("read-only"))
    }
    fn xmldecl(&self) -> XMLDecl {
        XMLDeclBuilder::new().build()
    }
    fn set_xmldecl(&mut self, _: XMLDecl) -> Result<(), Error> {
        Err(err("read-only"))
    }

    // ── Mutation / construction (read-only adapter → unsupported) ──

    fn new_element(&self, _: QName) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn new_text(&self, _: Rc<Value>) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn new_attribute(&self, _: QName, _: Rc<Value>) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn new_comment(&self, _: Rc<Value>) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn new_processing_instruction(&self, _: Rc<Value>, _: Rc<Value>) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn push(&mut self, _: Self) -> Result<(), Error> {
        Err(err("read-only"))
    }
    fn pop(&mut self) -> Result<(), Error> {
        Err(err("read-only"))
    }
    fn insert_before(&mut self, _: Self) -> Result<(), Error> {
        Err(err("read-only"))
    }
    fn add_attribute(&self, _: Self) -> Result<(), Error> {
        Err(err("read-only"))
    }
    fn shallow_copy(&self) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn deep_copy(&self) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn get_canonical(&self) -> Result<Self, Error> {
        Err(err("read-only"))
    }
    fn validate(&self, _: Schema) -> Result<(), ValidationError> {
        Err(ValidationError::SchemaError("not implemented".to_string()))
    }
}

impl PartialEq for ENode {
    fn eq(&self, other: &Self) -> bool {
        self.is_same(other)
    }
}

impl fmt::Debug for ENode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.loc {
            Loc::Node(id) => write!(f, "ENode(node {:?})", id),
            Loc::Attr { owner, idx } => write!(f, "ENode(attr {} of {:?})", idx, owner),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Correctness tests for the XPath-over-`scraper` adapter: real XPath 1.0
    //! queries must evaluate correctly over a `scraper`-parsed document via
    //! `ENode`, with no second parse.
    use super::*;
    use crate::engine::xpath_eval::evaluate;

    fn doc() -> Rc<Html> {
        Rc::new(Html::parse_document(
            r#"<html><body>
                <h1 class="headline">  Markets   Rally  </h1>
                <div class="product" data-sku="SKU-001">
                    <span class="vin">VIN: ABC123</span>
                    <span class="price">$42.00</span>
                </div>
                <div class="product" data-sku="SKU-002">
                    <span class="vin">VIN: XYZ789</span>
                    <span class="price">$99.00</span>
                </div>
            </body></html>"#,
        ))
    }

    #[test]
    fn element_text_query() {
        let d = doc();
        let r = evaluate(&d, "//span[@class='vin']/text()");
        assert_eq!(r, vec!["VIN: ABC123", "VIN: XYZ789"]);
    }

    #[test]
    fn attribute_query() {
        let d = doc();
        let r = evaluate(&d, "//div[@class='product']/@data-sku");
        assert_eq!(r, vec!["SKU-001", "SKU-002"]);
    }

    #[test]
    fn normalize_space_function() {
        let d = doc();
        let r = evaluate(&d, "normalize-space(//h1)");
        assert_eq!(r, vec!["Markets Rally"]);
    }

    #[test]
    fn contains_predicate() {
        let d = doc();
        let r = evaluate(&d, "//span[contains(., 'XYZ')]/text()");
        assert_eq!(r, vec!["VIN: XYZ789"]);
    }

    #[test]
    fn union_query() {
        let d = doc();
        let r = evaluate(&d, "//span[@class='price']/text() | //span[@class='vin']/text()");
        assert_eq!(r.len(), 4);
    }

    #[test]
    fn no_match_is_empty() {
        let d = doc();
        assert!(evaluate(&d, "//nonexistent").is_empty());
    }

    #[test]
    fn many_elements_is_fast() {
        // Guards the qualname-interner hotspot: name() is memoized, so a query
        // over a document with thousands of elements stays sub-second. Without
        // the QNAME_MEMO cache this took ~15s.
        let mut html = String::from("<html><body><div>");
        for _ in 0..20_000 {
            html.push_str("<span>x</span>");
        }
        html.push_str("</div></body></html>");
        let d = Rc::new(Html::parse_document(&html));
        let r = evaluate(&d, "//span");
        assert_eq!(r.len(), 20_000);
    }
}
