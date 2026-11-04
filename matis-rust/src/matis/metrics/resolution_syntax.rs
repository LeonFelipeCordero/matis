//! Replacement-decoded syntax and conservative lexical binding checks.
use super::adapters::resolution_policy::ResolutionAdapter;
use super::resolution_contract::{DeclarationLookup, FUNCTIONS};
use crate::domain::models::{ParsedFile, Symbol};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::sync::{Arc, Mutex, OnceLock};
use tree_sitter::{Node, Tree};
#[path = "resolution_bindings.rs"]
mod binding_index;
#[path = "pattern_bindings.rs"]
mod pattern_bindings;
pub use binding_index::NodeRef;
use binding_index::{BindingIndex, StoredBinding, walk_range};

pub const PARAMETERS: &[&str] = &[
    "parameters",
    "formal_parameters",
    "parameter_list",
    "function_value_parameters",
    "lambda_parameters",
    "closure_parameters",
    "inferred_parameters",
];
pub const IDENTIFIERS: &[&str] =
    &["identifier", "simple_identifier", "field_identifier", "shorthand_property_identifier_pattern"];

/// Return the first existing child field in `fields` order, or `None`.
pub fn field<'a>(node: Node<'a>, fields: &[&str]) -> Option<Node<'a>> {
    fields.iter().find_map(|name| node.child_by_field_name(name))
}
/// Collect direct named children in source order, excluding unnamed tokens.
pub fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}
/// Collect `node` and its named descendants in root-first source order without recursion.
pub fn walk(node: Node<'_>) -> Vec<Node<'_>> {
    let mut result = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        result.push(node);
        pending.extend(children(node).into_iter().rev());
    }
    result
}
/// Extract unique declared names from a binding pattern using matching source bytes.
/// Missing or unsupported patterns, member targets, and subscripts produce no names.
pub fn names(node: Option<Node<'_>>, source: &[u8]) -> BTreeSet<String> {
    let Some(node) = node else {
        return BTreeSet::new();
    };
    if IDENTIFIERS.contains(&node.kind()) {
        return [String::from_utf8_lossy(&source[node.byte_range()]).into_owned()].into();
    }
    if ["attribute", "member_expression", "field_expression", "subscript"].contains(&node.kind()) {
        return BTreeSet::new();
    }
    if node.kind() == "pair_pattern" {
        return names(field(node, &["value"]), source);
    }
    structured_names(node, source)
}
fn structured_names(node: Node<'_>, source: &[u8]) -> BTreeSet<String> {
    let mut cursor = node.walk();
    let declared: Vec<_> = node.children_by_field_name("name", &mut cursor).collect();
    if node.kind() == "parameter_declaration" && !declared.is_empty() {
        return declared.into_iter().flat_map(|n| names(Some(n), source)).collect();
    }
    if let Some(target) = field(node, &["pattern", "name", "declarator", "left"]) {
        return names(Some(target), source);
    }
    if ["typed_parameter", "default_parameter", "typed_default_parameter", "parameter", "variable_declaration"]
        .contains(&node.kind())
    {
        return names(children(node).into_iter().find(|n| IDENTIFIERS.contains(&n.kind())), source);
    }
    if node.kind().contains("pattern")
        || PARAMETERS.contains(&node.kind())
        || [
            "tuple",
            "list",
            "expression_list",
            "directly_assignable_expression",
            "parenthesized_declarator",
            "multi_variable_declaration",
        ]
        .contains(&node.kind())
    {
        return children(node).into_iter().flat_map(|n| names(Some(n), source)).collect();
    }
    BTreeSet::new()
}

pub type ReceiverScopeData = Vec<(Vec<NodeRef>, BTreeSet<String>)>;

#[derive(Default)]
struct NodeIndex {
    node_paths: BTreeMap<usize, NodeRef>,
    path_arena: Vec<(Option<usize>, u32)>,
    spans: BTreeMap<(usize, usize), NodeRef>,
}
impl NodeIndex {
    fn new(root: Option<Node<'_>>) -> Self {
        let mut result = Self::default();
        let Some(root) = root else { return result };
        let mut pending = vec![(root, None, 0)];
        while let Some((node, parent, child_index)) = pending.pop() {
            let index = result.path_arena.len();
            result.path_arena.push((parent, child_index));
            result.node_paths.insert(node.id(), NodeRef(index));
            pending.extend(child_paths(node, index));
        }
        result.spans = walk(root)
            .into_iter()
            .map(|node| ((node.start_byte(), node.end_byte()), result.node_paths[&node.id()]))
            .collect();
        result
    }
}
fn child_paths(node: Node<'_>, parent: usize) -> Vec<(Node<'_>, Option<usize>, u32)> {
    (0..node.child_count()).rev().map(|index| (node.child(index).unwrap(), Some(parent), index)).collect()
}
#[derive(Default)]
struct SyntaxCache {
    tree: OnceLock<Option<Tree>>,
    index: OnceLock<NodeIndex>,
    imports: OnceLock<Vec<(String, NodeRef)>>,
    import_nodes: OnceLock<BTreeMap<usize, Vec<NodeRef>>>,
    bindings: OnceLock<BindingIndex>,
    #[cfg(test)]
    dereferences: std::sync::atomic::AtomicUsize,
    written_members: OnceLock<HashSet<String>>,
}

#[derive(Clone)]
pub struct Syntax {
    pub language: String,
    pub file: Arc<ParsedFile>,
    pub adapter: ResolutionAdapter,
    pub source: Arc<Vec<u8>>,
    cache: Arc<SyntaxCache>,
    pub receiver_scope_cache: Arc<Mutex<BTreeMap<String, ReceiverScopeData>>>,
}
impl Syntax {
    /// Retain a file and policy with replacement-decoded UTF-8 source; defer parsing until needed.
    /// Clones share the syntax tree and lazy indexes.
    pub fn new(file: &ParsedFile, adapter: ResolutionAdapter) -> Self {
        let source = String::from_utf8_lossy(&file.source.source).into_owned().into_bytes();
        Self {
            language: file.source.language.clone(),
            file: Arc::new(file.clone()),
            adapter,
            source: Arc::new(source),
            cache: Arc::new(SyntaxCache::default()),
            receiver_scope_cache: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
    /// Parse and cache the source tree on first access; return `None` if parser setup or parsing fails.
    pub fn root(&self) -> Option<Node<'_>> {
        self.cache.tree.get_or_init(|| self.parse_tree()).as_ref().map(Tree::root_node)
    }
    fn parse_tree(&self) -> Option<Tree> {
        let language = tree_sitter_language_pack::get_language(&self.language).ok()?;
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&language).ok()?;
        parser.parse(self.source.as_slice(), None)
    }
    fn index(&self) -> &NodeIndex {
        self.cache.index.get_or_init(|| NodeIndex::new(self.root()))
    }
    /// Return a tree-relative handle that remains valid across shared cache clones.
    ///
    /// # Panics
    /// Panic if `node` does not belong to this syntax tree.
    pub fn reference(&self, node: Node<'_>) -> NodeRef {
        *self.index().node_paths.get(&node.id()).expect("Node must belong to this syntax tree")
    }
    /// Restore a handle from this tree or a shared clone; return `None` for an invalid path.
    pub fn dereference(&self, reference: &NodeRef) -> Option<Node<'_>> {
        #[cfg(test)]
        self.cache.dereferences.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let paths = &self.index().path_arena;
        let mut indices = Vec::new();
        let mut index = reference.0;
        while let Some((Some(parent), child)) = paths.get(index) {
            indices.push(*child);
            index = *parent;
        }
        paths.get(index)?;
        let mut node = self.root()?;
        for child in indices.into_iter().rev() {
            node = node.child(child)?;
        }
        Some(node)
    }
    /// Return root-first named nodes, or an empty vector if parsing failed.
    pub fn nodes(&self) -> Vec<Node<'_>> {
        self.root().map(walk).unwrap_or_default()
    }
    /// Return named nodes whose start bytes fall in the half-open `span`, in traversal order.
    pub fn nodes_in(&self, span: (usize, usize)) -> Vec<Node<'_>> {
        self.root().map(|root| walk_range(root, span)).unwrap_or_default()
    }
    /// Copy the replacement-decoded source text at `node`'s byte range.
    /// The node must use this source's byte offsets.
    pub fn text(&self, node: Node<'_>) -> String {
        String::from_utf8_lossy(&self.source[node.byte_range()]).into_owned()
    }
    /// Find an exact half-open UTF-8 byte span, or return `None`; later duplicate spans win.
    pub fn node(&self, start: usize, end: usize) -> Option<Node<'_>> {
        self.index().spans.get(&(start, end)).and_then(|reference| self.dereference(reference))
    }
    /// Find the nearest enclosing adapter scope, excluding `node`, then fall back to root or node.
    pub fn scope<'a>(&'a self, node: Node<'a>) -> Node<'a> {
        std::iter::successors(node.parent(), Node::parent)
            .find(|parent| self.adapter.scopes().contains(&parent.kind()))
            .unwrap_or_else(|| self.root().unwrap_or(node))
    }
    /// Return cached adapter-provided `(imported name, scope node)` entries.
    pub fn imports(&self) -> Vec<(String, Node<'_>)> {
        self.cache
            .imports
            .get_or_init(|| {
                self.adapter
                    .lexical_imports(self)
                    .into_iter()
                    .map(|(name, node)| (name, self.reference(node)))
                    .collect()
            })
            .iter()
            .filter_map(|(name, node)| Some((name.clone(), self.dereference(node)?)))
            .collect()
    }
    fn binding_index(&self) -> &BindingIndex {
        self.cache.bindings.get_or_init(|| {
            self.nodes()
                .into_iter()
                .flat_map(|node| self.node_bindings(node))
                .map(|(name, scope, node)| StoredBinding {
                    name,
                    scope: self.reference(scope),
                    node: self.reference(node),
                    scope_span: (scope.start_byte(), scope.end_byte()),
                    node_span: (node.start_byte(), node.end_byte()),
                })
                .collect()
        })
    }
    /// Return cached lexical bindings as `(name, scope, declaration)` tuples in traversal order.
    pub fn bindings(&self) -> Vec<(String, Node<'_>, Node<'_>)> {
        self.binding_index()
            .entries
            .iter()
            .filter_map(|binding| {
                Some((binding.name.clone(), self.dereference(&binding.scope)?, self.dereference(&binding.node)?))
            })
            .collect()
    }
    /// Return `(scope, declaration)` pairs for `name`, preserving binding order.
    pub fn bindings_named(&self, name: &str) -> Vec<(Node<'_>, Node<'_>)> {
        self.binding_index().named(name).map(|binding| self.restore_binding(binding)).collect()
    }
    /// Return named `(scope, declaration)` pairs whose half-open scope contains byte `position`.
    /// This range filter does not apply additional adapter visibility rules.
    pub fn bindings_at(&self, name: &str, position: usize) -> Vec<(Node<'_>, Node<'_>)> {
        self.binding_index()
            .named(name)
            .filter(|binding| binding.visible_at(position))
            .map(|binding| self.restore_binding(binding))
            .collect()
    }
    fn restore_binding(&self, binding: &StoredBinding) -> (Node<'_>, Node<'_>) {
        let scope = self.dereference(&binding.scope).expect("Cached binding scope must belong to this tree");
        let node = self.dereference(&binding.node).expect("Cached binding declaration must belong to this tree");
        (scope, node)
    }
    fn node_bindings<'a>(&'a self, node: Node<'a>) -> Vec<(String, Node<'a>, Node<'a>)> {
        if let Some(bindings) = pattern_bindings::scoped(self, node) {
            return bindings;
        }
        let scope = self.scope(node);
        if PARAMETERS.contains(&node.kind()) {
            return children(node)
                .into_iter()
                .flat_map(|child| self.binding_names(Some(child)))
                .map(|name| (name, scope, node))
                .collect();
        }
        if self.adapter.single_parameter_functions().contains(&node.kind()) {
            return field(node, &["parameter", "parameters"])
                .filter(|n| IDENTIFIERS.contains(&n.kind()))
                .map(|n| vec![(self.text(n), node, n)])
                .unwrap_or_default();
        }
        let (target, scope) = self.adapter.binding(self, node, scope);
        self.binding_names(target).into_iter().map(|name| (name, scope, node)).collect()
    }
    fn binding_names(&self, node: Option<Node<'_>>) -> BTreeSet<String> {
        if self.language == "rust" {
            pattern_bindings::rust_names(node, &self.source)
        } else {
            names(node, &self.source)
        }
    }
    /// Test half-open scope containment at byte `position` and adapter visibility restrictions.
    pub fn visible(&self, scope: Node<'_>, position: usize) -> bool {
        scope.start_byte() <= position && position < scope.end_byte() && self.adapter.visible(self, scope, position)
    }
    fn declaration_allowed(&self, lookup: &DeclarationLookup<'_>, allowed: &[Symbol]) -> bool {
        allowed.iter().any(|s| lookup.node.start_byte() <= s.start_byte && s.end_byte <= lookup.node.end_byte())
            || self.adapter.declaration_allowed(self, lookup)
    }
    /// Test for a conflicting visible binding of `name` at UTF-8 byte `position`.
    /// Exempt declarations containing `allowed` symbols and apply adapter import rules.
    pub fn shadowed(&self, name: &str, position: usize, allowed: &[Symbol]) -> bool {
        if self.adapter.import_shadowed(self, name, position) {
            return true;
        }
        self.binding_index().named(name).filter(|binding| binding.visible_at(position)).any(|binding| {
            let scope = self.dereference(&binding.scope).expect("Cached binding scope must belong to this tree");
            if !self.visible(scope, position) {
                return false;
            }
            let node = self.dereference(&binding.node).expect("Cached binding declaration must belong to this tree");
            !self.declaration_allowed(&DeclarationLookup { name, node, position }, allowed)
        })
    }
    /// Find one import on a one-based `line`, preferring normalized matches to optional `source`.
    /// Return `None` if no unique candidate remains.
    pub fn import_node(&self, line: usize, source: Option<&str>) -> Option<Node<'_>> {
        let index = self.cache.import_nodes.get_or_init(|| {
            let mut index: BTreeMap<usize, Vec<NodeRef>> = BTreeMap::new();
            for node in self.nodes() {
                if self.adapter.is_import_node_with_source(node, &self.source) {
                    index.entry(node.start_position().row + 1).or_default().push(self.reference(node));
                }
            }
            index
        });
        let mut candidates: Vec<_> =
            index.get(&line).into_iter().flatten().filter_map(|node| self.dereference(node)).collect();
        if let Some(source) = source {
            let exact: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|n| self.text(*n).trim().trim_end_matches(';') == source.trim().trim_end_matches(';'))
                .collect();
            if !exact.is_empty() {
                candidates = exact;
            }
        }
        if candidates.len() == 1 { Some(candidates[0]) } else { None }
    }
    /// Return the matched import's enclosing scope, or the tree root if no import is unique.
    pub fn import_scope(&self, line: usize, source: Option<&str>) -> Option<Node<'_>> {
        self.import_node(line, source).map(|n| self.scope(n)).or_else(|| self.root())
    }
    /// Test whether `name` is rebound directly in `symbol`'s class body, excluding nested scopes.
    pub fn class_rebound(&self, symbol: &Symbol, name: &str) -> bool {
        self.binding_index()
            .named(name)
            .filter(|binding| binding.contained_in((symbol.start_byte, symbol.end_byte)))
            .any(|binding| {
                let node =
                    self.dereference(&binding.node).expect("Cached binding declaration must belong to this tree");
                if self.adapter.rebound_declarations().contains(&node.kind()) {
                    return false;
                }
                let mut parent = node.parent();
                while let Some(candidate) = parent {
                    if FUNCTIONS.contains(&candidate.kind())
                        || ["class_definition", "class_declaration"].contains(&candidate.kind())
                    {
                        return candidate.start_byte() == symbol.start_byte && candidate.end_byte() == symbol.end_byte;
                    }
                    parent = candidate.parent();
                }
                false
            })
    }
    /// Cache and borrow member-write target spellings with all whitespace removed.
    pub fn written_members(&self) -> &HashSet<String> {
        self.cache.written_members.get_or_init(|| {
            self.nodes()
                .into_iter()
                .filter(|n| self.adapter.member_writes().contains(&n.kind()))
                .map(|n| {
                    self.adapter
                        .write_target(n)
                        .map(|n| self.text(n))
                        .unwrap_or_default()
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect()
                })
                .collect()
        })
    }
    /// Test whether a cached write target exactly matches `qualifier.name`.
    pub fn member_rebound(&self, qualifier: &str, name: &str) -> bool {
        self.written_members().contains(&format!("{qualifier}.{name}"))
    }
    /// Apply language-specific static-member detection to `symbol` using this file's syntax.
    pub fn static_symbol(&self, symbol: &Symbol) -> bool {
        self.adapter.static_symbol(self, symbol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::SourceFile;
    use crate::metrics::adapters::get_resolution_adapter;
    use std::path::PathBuf;

    fn parsed(language: &str, source: &[u8]) -> ParsedFile {
        ParsedFile {
            source: SourceFile {
                path: format!("a.{language}"),
                absolute_path: PathBuf::from(format!("a.{language}")),
                language: language.into(),
                source: source.to_vec(),
            },
            content_hash: String::new(),
            parse_error: None,
            symbols: vec![],
            imports: vec![],
            calls: vec![],
            inheritances: vec![],
        }
    }
    fn syntax(source: &[u8]) -> Syntax {
        Syntax::new(&parsed("python", source), get_resolution_adapter("python"))
    }
    #[test]
    fn constructors_and_clones_defer_parsing_for_every_language() {
        for language in ["python", "rust", "java", "javascript", "json", "csv"] {
            let syntax = Syntax::new(&parsed(language, b"x"), get_resolution_adapter(language));
            let cloned = syntax.clone();
            assert!(syntax.cache.tree.get().is_none());
            assert!(syntax.cache.index.get().is_none());
            assert!(Arc::ptr_eq(&syntax.cache, &cloned.cache));
            assert!(Arc::ptr_eq(&syntax.file, &cloned.file));
            assert!(Arc::ptr_eq(&syntax.source, &cloned.source));
            assert_eq!(syntax.root().unwrap().id(), cloned.root().unwrap().id());
            assert!(syntax.cache.index.get().is_none());
            let reference = syntax.reference(syntax.root().unwrap());
            assert!(std::ptr::eq(syntax.index(), cloned.index()));
            assert_eq!(cloned.dereference(&reference), syntax.root());
        }
    }
    #[test]
    fn cached_declarations_and_members_share_the_same_tree_across_clones() {
        let syntax = syntax(b"import sys\ndef f(value):\n return value\nservice.run = f\n");
        let declared = syntax.bindings();
        let imports = syntax.imports();
        let written = syntax.written_members();
        let cloned = syntax.clone();
        assert!(!declared.is_empty());
        assert!(!imports.is_empty());
        assert!(written.contains("service.run"));
        assert!(std::ptr::eq(written, cloned.written_members()));
        assert!(std::ptr::eq(syntax.cache.bindings.get().unwrap(), cloned.cache.bindings.get().unwrap()));
        assert!(std::ptr::eq(syntax.cache.imports.get().unwrap(), cloned.cache.imports.get().unwrap()));
        assert_eq!(declared, cloned.bindings());
        assert_eq!(imports, cloned.imports());
    }
    #[test]
    fn shadowing_filters_names_and_scope_ranges_before_restoring_nodes() {
        use std::sync::atomic::Ordering::Relaxed;
        let source = (0..500).map(|index| format!("def f{index}(value):\n return value\n")).collect::<String>();
        let syntax = syntax(source.as_bytes());
        let original: Vec<_> = syntax.nodes().into_iter().flat_map(|node| syntax.node_bindings(node)).collect();
        assert_eq!(syntax.bindings(), original);
        assert_eq!(syntax.binding_index().named("value").count(), 500);
        let counter = &syntax.cache.dereferences;
        counter.store(0, Relaxed);
        assert!(syntax.shadowed("value", source.rfind("return").unwrap(), &[]));
        assert_eq!(counter.load(Relaxed), 2);
        counter.store(0, Relaxed);
        assert!(!syntax.shadowed("missing", 0, &[]));
        assert!(!syntax.shadowed("value", source.len(), &[]));
        assert_eq!(counter.load(Relaxed), 0);
    }
    #[test]
    fn class_rebinding_filters_names_and_declaration_ranges_before_restoring_nodes() {
        use std::sync::atomic::Ordering::Relaxed;
        let source = format!(
            "class Target:\n value = 1\n{}",
            (0..500).map(|index| format!("def f{index}(value):\n return value\n")).collect::<String>()
        );
        let file = crate::parsing::parser::parse_file(parsed("python", source.as_bytes()).source).unwrap();
        let class = file.symbols.iter().find(|symbol| symbol.name == "Target").unwrap();
        let syntax = Syntax::new(&file, get_resolution_adapter("python"));
        assert_eq!(syntax.binding_index().named("value").count(), 501);
        let counter = &syntax.cache.dereferences;
        counter.store(0, Relaxed);
        assert!(syntax.class_rebound(class, "value"));
        assert_eq!(counter.load(Relaxed), 1);
        counter.store(0, Relaxed);
        assert!(!syntax.class_rebound(class, "missing"));
        assert_eq!(counter.load(Relaxed), 0);
    }
    #[test]
    fn bounded_walk_matches_full_named_walk_including_errors_and_boundaries() {
        let syntax = syntax(b"def first(x):\n return x\ndef broken(:\n return f()\n");
        let nodes = syntax.nodes();
        let spans = nodes.iter().map(|node| (node.start_byte(), node.end_byte())).chain([
            (0, 0),
            (0, 1),
            (1, syntax.source.len()),
            (syntax.source.len(), syntax.source.len()),
        ]);
        for span in spans {
            let full: Vec<_> = nodes
                .iter()
                .copied()
                .filter(|node| span.0 <= node.start_byte() && node.start_byte() < span.1)
                .collect();
            assert_eq!(syntax.nodes_in(span), full, "{span:?}");
        }
    }
    #[test]
    fn replacement_decoding_and_exact_span_lookup() {
        let syntax = syntax(b"value = '\xff'\n");
        assert_eq!(syntax.source.as_slice(), "value = '\u{fffd}'\n".as_bytes());
        let identifier = syntax.nodes().into_iter().find(|n| n.kind() == "identifier").unwrap();
        assert_eq!(syntax.node(identifier.start_byte(), identifier.end_byte()), Some(identifier));
        assert!(syntax.node(identifier.start_byte(), identifier.end_byte() - 1).is_none());
    }
    #[test]
    fn stable_node_handles_preserve_duplicate_spans_across_clones() {
        let syntax = syntax(b"x");
        let original = syntax.root().unwrap();
        let duplicates: Vec<_> = syntax.nodes().into_iter().filter(|node| node.byte_range() == (0..1)).collect();
        assert_eq!(duplicates.iter().map(|node| node.kind()).collect::<Vec<_>>(), ["module", "identifier"]);
        let reference = syntax.reference(original);
        let clone = syntax.clone();
        let restored = clone.dereference(&reference).unwrap();
        assert_eq!(restored.kind(), "module");
        assert_eq!(clone.reference(clone.root().unwrap()).0, 0);
        assert_eq!(clone.reference(restored).0, reference.0);
        assert_eq!(syntax.node(0, 1).unwrap().kind(), "identifier");
    }
    #[test]
    fn import_source_disambiguates_same_line_and_unknown_source_keeps_ambiguity() {
        let syntax = syntax(b"import first; import second\n");
        assert!(syntax.import_node(1, None).is_none());
        assert!(syntax.import_node(1, Some("import unknown")).is_none());
        assert_eq!(syntax.text(syntax.import_node(1, Some(" import first; ")).unwrap()), "import first");
        assert_eq!(syntax.import_scope(2, None), syntax.root());
        let cloned = syntax.clone();
        assert!(std::ptr::eq(syntax.cache.import_nodes.get().unwrap(), cloned.cache.import_nodes.get().unwrap()));
        assert_eq!(cloned.import_node(1, Some("import second")), syntax.import_node(1, Some("import second")));
        syntax.cache.dereferences.store(0, std::sync::atomic::Ordering::Relaxed);
        assert!(syntax.import_node(500, None).is_none());
        assert_eq!(syntax.cache.dereferences.load(std::sync::atomic::Ordering::Relaxed), 0);
    }
    #[test]
    fn nearest_parent_scope_and_half_open_visibility() {
        let syntax = syntax(b"def outer():\n def inner(value):\n  return value\n");
        let inner = syntax.nodes().into_iter().filter(|n| n.kind() == "function_definition").nth(1).unwrap();
        let outer = syntax.scope(inner);
        assert_eq!(outer.kind(), "function_definition");
        assert!(syntax.visible(inner, inner.start_byte()));
        assert!(!syntax.visible(inner, inner.end_byte()));
        assert!(syntax.shadowed("value", inner.end_byte() - 1, &[]));
        assert!(!syntax.shadowed("value", 0, &[]));
    }
    #[test]
    fn member_write_matching_removes_whitespace_and_keeps_full_name() {
        let syntax = syntax(b"service . run = replacement\n");
        assert!(syntax.member_rebound("service", "run"));
        assert!(!syntax.member_rebound("service", "runner"));
    }
    #[test]
    fn pattern_names_exclude_types_and_members() {
        let source = b"def f(x: int, y=1):\n a.b = 1\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("python").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let nodes = walk(tree.root_node());
        let parameters = nodes.iter().find(|n| n.kind() == "parameters").unwrap();
        assert_eq!(names(Some(*parameters), source), ["x".to_owned(), "y".to_owned()].into());
        let attribute = nodes.iter().find(|n| n.kind() == "attribute").unwrap();
        assert!(names(Some(*attribute), source).is_empty());
    }
}
