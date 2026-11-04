//! Shared extraction syntax: calls, declaration carriers, imports, and metadata.
use crate::{
    domain::models::SourceFile,
    syntax::{children, field, named_children, text, walk},
};
use anyhow::{Result, anyhow};
use tree_sitter::{Node, Tree};

pub const CALL_NODE_TYPES: &[&str] = &[
    "call",
    "call_expression",
    "function_call",
    "invocation_expression",
    "method_invocation",
    "new_expression",
    "object_creation_expression",
    "remote_call",
];
pub const REFERENCES: &[&str] = &["method_reference", "callable_reference"];
pub const FALLBACK_CALL_TYPES: &[&str] = &[
    "call",
    "call_expression",
    "function_call",
    "invocation_expression",
    "method_invocation",
    "new_expression",
    "object_creation_expression",
    "remote_call",
    "method_reference",
    "callable_reference",
];

#[derive(Clone)]
pub struct CallSite<'tree> {
    pub node: Node<'tree>,
    pub offset: isize,
    pub opaque: bool,
    pub parents: Vec<(Node<'tree>, &'tree [u8])>,
}
impl<'tree> CallSite<'tree> {
    /// Create a site at the node's original byte offsets, without synthetic payload context.
    pub fn new(node: Node<'tree>) -> Self {
        Self { node, offset: 0, opaque: false, parents: vec![] }
    }
}
pub type CallTarget = (Option<String>, bool, Option<String>);
pub type CallVisitor<'a> = dyn FnMut(CallSite<'_>, &[u8]) + 'a;

/// Parse in-memory source bytes with the requested language-pack grammar.
/// Syntax errors remain in the returned tree rather than causing an error.
///
/// # Errors
/// Returns an error if grammar lookup, parser configuration, or parsing fails.
pub fn parse(language: &str, source: &[u8]) -> Result<Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language(language)?)?;
    parser.parse(source, None).ok_or_else(|| anyhow!("Parsing cancelled for {language}"))
}

pub struct ParsedSyntax<'a> {
    pub source: &'a SourceFile,
    pub root: Node<'a>,
}

/// Return `REFERENCES` for method/callable references and `CALLS` for other sites.
pub fn relationship(site: &CallSite<'_>) -> &'static str {
    if REFERENCES.contains(&site.node.kind()) { "REFERENCES" } else { "CALLS" }
}

/// Return trimmed member text from `source`, falling back to the complete target.
/// `source` must correspond to the target's syntax tree.
pub fn call_member(target: Node<'_>, source: &[u8]) -> String {
    text(Some(field(target, &["property", "attribute", "field", "name", "method", "right"]).unwrap_or(target)), source)
        .trim()
        .to_owned()
}

fn reference_target(node: Node<'_>, source: &[u8]) -> CallTarget {
    let parts: Vec<_> = named_children(node).into_iter().filter(|c| c.kind() != "type_arguments").collect();
    if parts.is_empty() {
        return (None, false, None);
    }
    let constructor = children(node).iter().any(|c| c.kind() == "new");
    let qualifier = (parts.len() > 1 || constructor).then(|| text(parts.first().copied(), source));
    let name = if constructor { "new".to_owned() } else { text(parts.last().copied(), source) };
    (Some(name), qualifier.is_none(), qualifier)
}

fn callee_node(node: Node<'_>) -> Option<Node<'_>> {
    let target = field(node, &["function", "callee", "target", "name", "method", "constructor", "type"])
        .or_else(|| node.named_child(0))?;
    if matches!(target.kind(), "generic_function" | "template_function" | "parameterized_expression") {
        Some(field(target, &["function", "name", "expression"]).or_else(|| target.named_child(0)).unwrap_or(target))
    } else {
        Some(target)
    }
}

/// Extract the optional target name, bare-name flag, and optional syntactic qualifier.
/// Handle constructor references and dotted/scoped names without resolving symbols.
/// Use `member` for other target names; return `(None, false, None)` if no callee exists.
/// `source` must correspond to the node's syntax tree.
pub fn call_target(node: Node<'_>, source: &[u8], member: fn(Node<'_>, &[u8]) -> String) -> CallTarget {
    if REFERENCES.contains(&node.kind()) {
        return reference_target(node, source);
    }
    let Some(target) = callee_node(node) else {
        return (None, false, None);
    };
    let bare = field(node, &["object", "receiver", "scope"]).is_none()
        && matches!(target.kind(), "identifier" | "simple_identifier" | "type_identifier")
        && target.named_child_count() == 0;
    let raw = match field(node, &["object", "receiver"]) {
        Some(object) => format!("{}.{}", text(Some(object), source), text(Some(target), source)),
        None => text(Some(target), source),
    };
    static DOTTED: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    if DOTTED
        .get_or_init(|| regex::Regex::new(r"^([\p{L}\p{N}_$]+(?:\.|::)[\p{L}\p{N}_.$:]+)$").unwrap())
        .is_match(&raw)
    {
        let separator = if raw.contains("::") { "::" } else { "." };
        let (qualifier, terminal) = raw.rsplit_once(separator).unwrap();
        return (Some(terminal.to_owned()), false, Some(qualifier.to_owned()));
    }
    let name = member(target, source);
    ((!name.is_empty()).then_some(name), bare, None)
}

/// Extract the site's target name, bare-name flag, and qualifier using shared syntax.
/// `source` must match the site's tree; offsets and synthetic scope data are not used.
pub fn normalize_call(site: &CallSite<'_>, source: &[u8]) -> CallTarget {
    call_target(site.node, source, call_member)
}

/// Return optional visibility and annotation text from the node's direct modifiers.
/// Recognize visibility modifier nodes and values in `words`, using matching `source` bytes.
pub fn modifier_metadata(node: Node<'_>, source: &[u8], words: &[&str]) -> (Option<String>, Vec<String>) {
    let modifiers = named_children(node).into_iter().find(|child| child.kind() == "modifiers");
    let mut visibility = None;
    let mut decorators = vec![];
    for child in modifiers.map(children).unwrap_or_default() {
        let value = text(Some(child), source);
        if child.kind() == "visibility_modifier" || words.contains(&value.as_str()) {
            visibility = Some(value.clone());
        }
        if child.kind().contains("annotation") {
            decorators.push(value);
        }
    }
    (visibility, decorators)
}

/// Return explicit visibility and annotations using the shared visibility vocabulary.
/// `source` must correspond to the node's syntax tree; no default visibility is inferred.
pub fn metadata(node: Node<'_>, source: &[u8]) -> (Option<String>, Vec<String>) {
    modifier_metadata(node, source, &["public", "private", "protected", "internal", "pub"])
}

#[derive(Clone)]
pub struct DeclarationCarrier<'tree> {
    pub node: Node<'tree>,
    pub name: Option<String>,
    pub kind: Option<String>,
    pub decorators: Vec<String>,
}

/// Keep the declaration node and read its optional `name` field from matching `source` bytes.
/// Leave kind and decorators unset.
pub fn named_carrier<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    DeclarationCarrier {
        node,
        name: node.child_by_field_name("name").map(|name| text(Some(name), source)),
        kind: None,
        decorators: vec![],
    }
}

/// Mark the node as a function carrier, preferring a nonempty `binding` to its own name.
/// Read the fallback name from `source`, which must match the node's tree.
pub fn callback_carrier<'tree>(node: Node<'tree>, binding: Option<String>, source: &[u8]) -> DeclarationCarrier<'tree> {
    let mut carrier = named_carrier(node, source);
    carrier.name = binding.filter(|s| !s.is_empty()).or(carrier.name);
    carrier.kind = Some("function".to_owned());
    carrier
}

fn expression_children(node: Node<'_>) -> Vec<Node<'_>> {
    named_children(node).into_iter().filter(|child| child.kind() != "comment").collect()
}

/// Return a callback's variable, assignment target, or pair key from matching `source` bytes.
/// Unwrap single-expression parentheses; return `None` for unsupported parent shapes.
pub fn callback_binding(mut node: Node<'_>, source: &[u8]) -> Option<String> {
    while let Some(parent) = node.parent().filter(|p| p.kind() == "parenthesized_expression") {
        if expression_children(parent) != vec![node] {
            return None;
        }
        node = parent;
    }
    let parent = node.parent()?;
    if !matches!(parent.kind(), "variable_declarator" | "assignment_expression" | "pair") {
        return None;
    }
    let value_field = if parent.kind() == "assignment_expression" { "right" } else { "value" };
    if parent.child_by_field_name(value_field) != Some(node) {
        return None;
    }
    let name_field = match parent.kind() {
        "pair" => "key",
        "assignment_expression" => "left",
        _ => "name",
    };
    let name = parent.child_by_field_name(name_field)?;
    Some(text(Some(name.child_by_field_name("property").unwrap_or(name)), source))
}

pub struct ImportPolicy<'a> {
    pub node_type: Option<&'a str>,
    pub accept: fn(Node<'_>) -> bool,
}

/// Add imports accepted by `policy` from the root and its matching source bytes.
/// Entries contain text and one-based lines. Deduplicate additions by trimmed text and line,
/// preserve native entries, and sort by line. Return `imports` unchanged if no node type is set.
pub fn supplement_imports(
    (root, source): (Node<'_>, &[u8]),
    mut imports: Vec<(String, usize)>,
    policy: ImportPolicy<'_>,
) -> Vec<(String, usize)> {
    let Some(node_type) = policy.node_type else {
        return imports;
    };
    let mut seen: std::collections::HashSet<_> =
        imports.iter().map(|(value, line)| (value.trim().to_owned(), *line)).collect();
    for node in walk(root).filter(|node| node.kind() == node_type && (policy.accept)(*node)) {
        let value = text(Some(node), source).trim_end().to_owned();
        let line = node.start_position().row + 1;
        if seen.insert((value.trim().to_owned(), line)) {
            imports.push((value, line));
        }
    }
    imports.sort_by_key(|(_, line)| *line);
    imports
}

pub use super::inheritance::{DeclarationHeader, InheritancePolicy, inheritance_names};
