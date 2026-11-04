//! Header-only inheritance extraction; bodies and generic arguments are excluded.
use super::syntax::parse;
use crate::syntax::{children, field, named_children, text};
use tree_sitter::Node;

const BODY_NODES: &[&str] = &[
    "class_body",
    "interface_body",
    "field_declaration_list",
    "declaration_list",
    "block",
    "statement_block",
    "function_body",
];
const NAME_NODES: &[&str] = &[
    "identifier",
    "type_identifier",
    "namespace_identifier",
    "scoped_identifier",
    "scoped_type_identifier",
    "qualified_identifier",
    "attribute",
    "member_expression",
    "nested_type_identifier",
    "generic_type",
    "template_type",
    "user_type",
    "simple_identifier",
    "property_identifier",
    "nested_identifier",
    "subscript",
    "constructor_invocation",
    "call_expression",
];

#[derive(Clone, Copy)]
pub struct InheritancePolicy {
    pub clauses: &'static [&'static str],
    pub complete_header: bool,
    pub superclass_field: Option<&'static str>,
    pub heritage_clause: Option<&'static str>,
    pub kotlin_base: bool,
}
impl Default for InheritancePolicy {
    /// Use the broad fallback clauses, superclass field, and class heritage node.
    /// Do not complete headers or enable Kotlin-specific base-name handling.
    fn default() -> Self {
        Self {
            clauses: &[
                "superclass",
                "super_interfaces",
                "extends_interfaces",
                "extends_type_clause",
                "extends_clause",
                "implements_clause",
                "base_class_clause",
                "delegation_specifier",
            ],
            complete_header: false,
            superclass_field: Some("superclasses"),
            heritage_clause: Some("class_heritage"),
            kotlin_base: false,
        }
    }
}
impl InheritancePolicy {
    /// Create a policy for `clauses` with all optional extraction behavior disabled.
    pub const fn new(clauses: &'static [&'static str]) -> Self {
        Self { clauses, complete_header: false, superclass_field: None, heritage_clause: None, kotlin_base: false }
    }
}

pub struct DeclarationHeader<'a> {
    pub language: &'a str,
    pub kind: &'a str,
    pub signature: &'a str,
}

fn base_target(node: Node<'_>) -> Option<Node<'_>> {
    let fields = if matches!(node.kind(), "subscript" | "call_expression" | "constructor_invocation") {
        &["value", "function"]
    } else {
        &["name", "type"]
    };
    field(node, fields).or_else(|| node.named_child(0))
}

fn base_name(node: Node<'_>, source: &[u8], policy: InheritancePolicy) -> Option<String> {
    if policy.kotlin_base && node.kind() == "user_type" {
        let name = named_children(node)
            .into_iter()
            .filter(|c| c.kind() == "type_identifier")
            .map(|c| text(Some(c), source))
            .collect::<Vec<_>>()
            .join(".");
        if !name.is_empty() {
            return Some(name);
        }
    }
    if !NAME_NODES.contains(&node.kind()) {
        return None;
    }
    if matches!(
        node.kind(),
        "subscript" | "call_expression" | "constructor_invocation" | "generic_type" | "template_type"
    ) {
        return base_target(node).and_then(|n| base_name(n, source, policy));
    }
    if node.named_child_count() == 0 {
        return Some(text(Some(node), source));
    }
    let name: String = children(node)
        .into_iter()
        .map(|child| {
            if matches!(child.kind(), "." | "::") {
                text(Some(child), source)
            } else {
                base_name(child, source, policy).unwrap_or_default()
            }
        })
        .collect();
    (!name.is_empty()).then_some(name)
}

fn clause_names(node: Node<'_>, source: &[u8], policy: InheritancePolicy) -> Vec<String> {
    if matches!(
        node.kind(),
        "type_arguments" | "type_argument_list" | "template_argument_list" | "value_arguments" | "access_specifier"
    ) {
        return vec![];
    }
    if let Some(name) = base_name(node, source, policy).filter(|s| !s.is_empty()) {
        return vec![name];
    }
    named_children(node).into_iter().flat_map(|child| clause_names(child, source, policy)).collect()
}

fn header_names(root: Node<'_>, source: &[u8], policy: InheritancePolicy) -> Vec<String> {
    let mut pending = vec![root];
    let mut names = vec![];
    while let Some(node) = pending.pop() {
        if BODY_NODES.contains(&node.kind()) {
            continue;
        }
        if let Some(bases) = policy.superclass_field.and_then(|name| node.child_by_field_name(name)) {
            names.extend(
                named_children(bases)
                    .into_iter()
                    .filter_map(|base| base_name(base, source, policy))
                    .filter(|s| !s.is_empty()),
            );
        }
        let has_clause = named_children(node).iter().any(|c| policy.clauses.contains(&c.kind()));
        if policy.clauses.contains(&node.kind()) || (Some(node.kind()) == policy.heritage_clause && !has_clause) {
            names.extend(clause_names(node, source, policy));
            continue;
        }
        pending.extend(named_children(node).into_iter().rev());
    }
    names
}

/// Extract syntactic base names from class, interface, struct, or trait headers.
/// Exclude bodies and generic arguments; `policy` can retry a header with an empty body.
/// Return an empty list for other kinds, empty signatures, or parser failures.
pub fn inheritance_names(header: DeclarationHeader<'_>, policy: InheritancePolicy) -> Vec<String> {
    if !["class", "interface", "struct", "trait"].contains(&header.kind) || header.signature.is_empty() {
        return vec![];
    }
    let Ok(tree) = parse(header.language, header.signature.as_bytes()) else {
        return vec![];
    };
    if tree.root_node().has_error() && policy.complete_header {
        let completed = format!("{} {{}}", header.signature);
        if let Ok(tree) = parse(header.language, completed.as_bytes())
            && !tree.root_node().has_error()
        {
            return header_names(tree.root_node(), completed.as_bytes(), policy);
        }
    }
    header_names(tree.root_node(), header.signature.as_bytes(), policy)
}
