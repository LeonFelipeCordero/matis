//! Merge native metadata and bundled tags on canonical syntax-tree carriers.
use super::{
    adapters::{LanguageAdapter, get_adapter},
    queries::tags_query,
    syntax::{DeclarationCarrier, ParsedSyntax},
};
use crate::syntax::text;
use anyhow::Result;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Node, Query, QueryCapture, QueryCursor};
use tree_sitter_language_pack::{Span, StructureItem, StructureKind, get_language};

type Declaration<'a> = (Node<'a>, StructureItem);
type DeclarationKey = (usize, Option<String>);
const KINDS: &[(&str, StructureKind)] = &[
    ("function", StructureKind::Function),
    ("method", StructureKind::Method),
    ("class", StructureKind::Class),
    ("struct", StructureKind::Struct),
    ("interface", StructureKind::Interface),
    ("enum", StructureKind::Enum),
    ("module", StructureKind::Module),
    ("trait", StructureKind::Trait),
    ("impl", StructureKind::Impl),
    ("namespace", StructureKind::Namespace),
];
const DEFINITION_KINDS: &[&str] =
    &["class", "interface", "function", "method", "module", "struct", "trait", "enum", "type"];

/// Return the lowercase graph kind, preserving `type` and `constructor` custom names.
/// Other custom kinds use the lowercase JSON representation of `kind`.
pub fn kind_name(kind: &StructureKind) -> String {
    match kind {
        StructureKind::Other(name) if matches!(name.as_str(), "type" | "constructor") => name.clone(),
        StructureKind::Other(_) => serde_json::to_string(kind).unwrap().to_lowercase(),
        _ => format!("{kind:?}").to_lowercase(),
    }
}

fn structure_kind(kind: &str) -> StructureKind {
    KINDS
        .iter()
        .find(|(name, _)| *name == kind)
        .map(|(_, kind)| kind.clone())
        .unwrap_or_else(|| StructureKind::Other(kind.to_owned()))
}

/// Return the cached query combining bundled tags and adapter declaration captures.
/// Cache successful lookups, including `None` when neither source provides query text.
///
/// # Errors
/// Returns an error for an unsupported language, failed grammar lookup, or invalid query.
pub fn declaration_query(language: &str) -> Result<Option<Arc<Query>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<Arc<Query>>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    if let Some(query) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(language) {
        return Ok(query.clone());
    }
    let adapter = get_adapter(language)?;
    let source = format!("{}{}", tags_query(language).unwrap_or_default(), adapter.declaration_query());
    let query = if source.is_empty() { None } else { Some(Arc::new(Query::new(&get_language(language)?, &source)?)) };
    cache.lock().unwrap_or_else(|e| e.into_inner()).insert(language.to_owned(), query.clone());
    Ok(query)
}

fn span(node: Node<'_>) -> Span {
    Span {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: node.start_position().row,
        start_column: node.start_position().column,
        end_line: node.end_position().row,
        end_column: node.end_position().column,
    }
}

struct Extraction<'a> {
    source: &'a [u8],
    adapter: LanguageAdapter<'a>,
}
struct CapturedDeclaration<'a> {
    node: Node<'a>,
    kind: String,
    name: Option<String>,
    has_name: bool,
}
#[derive(Default)]
struct CapturedDeclarations<'a> {
    entries: Vec<Declaration<'a>>,
    positions: HashMap<usize, usize>,
}

impl Extraction<'_> {
    fn structure_item(&self, carrier: &DeclarationCarrier<'_>, kind: &str) -> StructureItem {
        let (visibility, decorators) = self.adapter.metadata(carrier.node, self.source);
        StructureItem {
            name: carrier.name.clone(),
            kind: structure_kind(kind),
            signature: Some(text(Some(carrier.node), self.source)),
            visibility,
            decorators,
            span: span(carrier.node),
            ..Default::default()
        }
    }
    fn captured_declarations<'a>(&self, root: Node<'a>, query: &Query) -> Vec<Declaration<'a>> {
        let mut declarations = CapturedDeclarations::default();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(query, root, self.source);
        while let Some(found) = matches.next() {
            self.add_captured_match(found.captures(), query, &mut declarations);
        }
        declarations.entries
    }
    fn add_captured_match<'a>(
        &self,
        captures: &[QueryCapture<'a>],
        query: &Query,
        declarations: &mut CapturedDeclarations<'a>,
    ) {
        let names = query.capture_names();
        let Some(definition) = captures.iter().find(|c| definition_kind(names[c.index as usize]).is_some()) else {
            return;
        };
        let capture_name = names[definition.index as usize];
        let kind = definition_kind(capture_name).unwrap();
        let name_node = captures.iter().find(|capture| names[capture.index as usize] == "name").map(|c| c.node);
        let name = name_node.map(|node| text(Some(node), self.source));
        for capture in captures.iter().filter(|c| names[c.index as usize] == capture_name) {
            let capture = CapturedDeclaration {
                node: capture.node,
                kind: kind.to_owned(),
                name: name.clone(),
                has_name: name_node.is_some(),
            };
            self.add_capture(declarations, capture);
        }
    }
    fn add_capture<'a>(&self, declarations: &mut CapturedDeclarations<'a>, capture: CapturedDeclaration<'a>) {
        if let Some(index) = declarations.positions.get(&capture.node.id()).copied() {
            if capture.has_name {
                // Metadata is pure for this node/source; only name and kind depend on the capture.
                declarations.entries[index].1.name = capture.name;
                declarations.entries[index].1.kind = structure_kind(&capture.kind);
            }
            return;
        }
        let carrier = DeclarationCarrier { node: capture.node, name: capture.name, kind: None, decorators: vec![] };
        let item = self.structure_item(&carrier, &capture.kind);
        declarations.positions.insert(capture.node.id(), declarations.entries.len());
        declarations.entries.push((capture.node, item));
    }
    fn canonical_declaration<'a>(&self, node: Node<'a>, mut item: StructureItem) -> Option<Declaration<'a>> {
        let carrier = self.adapter.normalize_declaration(node, self.source)?;
        let (visibility, decorators) = self.adapter.metadata(carrier.node, self.source);
        let changed_span =
            carrier.node.start_byte() != item.span.start_byte || carrier.node.end_byte() != item.span.end_byte;
        item.name = nonempty(carrier.name).or_else(|| nonempty(item.name));
        if let Some(kind) = carrier.kind {
            item.kind = structure_kind(&kind);
        }
        if changed_span {
            item.span = span(carrier.node);
            item.signature = Some(text(Some(carrier.node), self.source));
        }
        item.visibility = nonempty(item.visibility).or(visibility);
        if item.decorators.is_empty() {
            item.decorators = if carrier.decorators.is_empty() { decorators } else { carrier.decorators };
        }
        item.children.clear();
        Some((carrier.node, item))
    }
}

fn definition_kind(capture: &str) -> Option<&str> {
    let kind = capture.strip_prefix("definition.")?;
    DEFINITION_KINDS.contains(&kind).then_some(kind)
}

fn native_declarations<'a>(root: Node<'a>, items: &[StructureItem], declarations: &mut Vec<Declaration<'a>>) {
    for item in items {
        let end = item.span.start_byte.max(item.span.end_byte.saturating_sub(1));
        if let Some(node) = root.descendant_for_byte_range(item.span.start_byte, end) {
            declarations.push((node, without_children(item)));
        }
        native_declarations(root, &item.children, declarations);
    }
}

fn without_children(item: &StructureItem) -> StructureItem {
    StructureItem {
        kind: item.kind.clone(),
        name: item.name.clone(),
        visibility: item.visibility.clone(),
        span: item.span.clone(),
        children: vec![],
        decorators: item.decorators.clone(),
        doc_comment: item.doc_comment.clone(),
        signature: item.signature.clone(),
        body_span: item.body_span.clone(),
    }
}

fn nonempty(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.is_empty())
}

fn declaration_key(node: Node<'_>, item: &StructureItem, adapter: LanguageAdapter<'_>) -> DeclarationKey {
    (node.id(), if adapter.callback_types().contains(&node.kind()) { None } else { item.name.clone() })
}

fn merge_item(mut native: StructureItem, tagged: &StructureItem) -> StructureItem {
    native.name = nonempty(native.name).or_else(|| tagged.name.clone());
    if tagged.kind == StructureKind::Interface {
        native.kind = tagged.kind.clone();
    }
    native.signature = nonempty(native.signature).or_else(|| tagged.signature.clone());
    native.visibility = nonempty(native.visibility).or_else(|| tagged.visibility.clone());
    if native.decorators.is_empty() {
        native.decorators = tagged.decorators.clone();
    }
    native
}

#[derive(Default)]
struct DeclarationSet<'a> {
    entries: Vec<Declaration<'a>>,
    positions: HashMap<DeclarationKey, usize>,
}
impl<'a> DeclarationSet<'a> {
    fn insert(&mut self, (node, item): Declaration<'a>, adapter: LanguageAdapter<'_>, native: bool) {
        let key = declaration_key(node, &item, adapter);
        if let Some(index) = self.positions.get(&key).copied() {
            let item = if native { merge_item(item, &self.entries[index].1) } else { item };
            self.entries[index] = (node, item);
        } else {
            self.positions.insert(key, self.entries.len());
            self.entries.push((node, item));
        }
    }
}

fn merge_declarations<'a>(
    syntax: &ParsedSyntax<'a>,
    query: &Query,
    existing: &[StructureItem],
) -> Vec<Declaration<'a>> {
    let extraction =
        Extraction { source: &syntax.source.source, adapter: LanguageAdapter { language: &syntax.source.language } };
    let mut declarations = DeclarationSet::default();
    for (node, item) in extraction.captured_declarations(syntax.root, query) {
        if let Some(declaration) = extraction.canonical_declaration(node, item) {
            declarations.insert(declaration, extraction.adapter, false);
        }
    }
    let mut native = vec![];
    native_declarations(syntax.root, existing, &mut native);
    for (node, item) in native {
        if let Some(declaration) = extraction.canonical_declaration(node, item) {
            declarations.insert(declaration, extraction.adapter, true);
        }
    }
    declarations.entries
}

fn build_item(index: usize, declarations: &[Declaration<'_>], children: &[Vec<usize>]) -> StructureItem {
    let mut item = declarations[index].1.clone();
    item.children = children[index].iter().map(|index| build_item(*index, declarations, children)).collect();
    item
}

fn declared_parent(node: Node<'_>, by_node: &HashMap<usize, usize>) -> Option<usize> {
    std::iter::successors(node.parent(), |parent| parent.parent()).find_map(|parent| by_node.get(&parent.id()).copied())
}

fn structure_hierarchy(declarations: Vec<Declaration<'_>>) -> Vec<StructureItem> {
    let by_node: HashMap<_, _> = declarations.iter().enumerate().map(|(i, (node, _))| (node.id(), i)).collect();
    let mut order: Vec<_> = (0..declarations.len()).collect();
    order.sort_by_key(|i| declarations[*i].0.start_byte());
    let mut roots = vec![];
    let mut children = vec![vec![]; declarations.len()];
    for index in order {
        if let Some(parent) = declared_parent(declarations[index].0, &by_node) {
            children[parent].push(index);
        } else {
            roots.push(index);
        }
    }
    roots.into_iter().map(|index| build_item(index, &declarations, &children)).collect()
}

/// Merge native `existing` items with AST declarations from the supplied syntax tree.
/// Rebuild source-ordered children under their nearest declared AST ancestor without
/// mutating native items. Preserve native metadata except carrier name, kind, and span corrections.
/// Return a copy of `existing` when no declaration query is available.
///
/// # Errors
/// Returns an error if language validation, grammar lookup, or query compilation fails.
pub fn extract_structure(syntax: &ParsedSyntax<'_>, existing: &[StructureItem]) -> Result<Vec<StructureItem>> {
    let Some(query) = declaration_query(&syntax.source.language)? else {
        return Ok(existing.to_vec());
    };
    Ok(structure_hierarchy(merge_declarations(syntax, &query, existing)))
}

#[cfg(test)]
#[path = "../../../tests/unit/parsing/structure_tests.rs"]
mod tests;
