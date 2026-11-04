use crate::parsing::syntax::{
    self, CallSite, CallTarget, CallVisitor, DeclarationCarrier, callback_binding, callback_carrier, named_carrier,
};
use crate::syntax::{field, named_children, text, walk};
use tree_sitter::Node;
pub const CALLBACK_TYPES: &[&str] = &["arrow_function", "function_expression", "generator_function"];
pub const QUERY: &str = "\n[(arrow_function) (function_expression) (generator_function)] @definition.function\n";

/// Test whether the node names an opening or self-closing JSX tag, including native tags.
pub fn is_jsx_site(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        matches!(parent.kind(), "jsx_opening_element" | "jsx_self_closing_element")
            && field(parent, &["name"]) == Some(node)
    })
}

fn component(node: Node<'_>, source: &[u8]) -> bool {
    if node.kind() == "member_expression" {
        return true;
    }
    let name = text(Some(node), source);
    node.kind() == "identifier"
        && !name.is_empty()
        && !name.starts_with(|c: char| c.is_ascii_lowercase())
        && !name.contains('-')
}

/// Invoke `visitor` for calls, constructors, and component JSX tags in depth-first source order.
/// Exclude native JSX tags; pass each site with the supplied source bytes matching `root`.
pub fn visit_calls(root: Node<'_>, source: &[u8], visitor: &mut CallVisitor<'_>) {
    for node in walk(root) {
        if matches!(node.kind(), "call_expression" | "new_expression") || is_jsx_site(node) && component(node, source) {
            visitor(CallSite::new(node), source);
        }
    }
}

/// Return the optional target name, bare-name flag, and qualifier using matching source bytes.
/// Split JSX member names at the final dot; use shared syntax for other sites.
pub fn normalize_call(site: &CallSite<'_>, source: &[u8]) -> CallTarget {
    if !is_jsx_site(site.node) {
        return syntax::normalize_call(site, source);
    }
    let name = text(Some(site.node), source);
    match name.rsplit_once('.') {
        Some((qualifier, member)) => (Some(member.to_owned()), false, Some(qualifier.to_owned())),
        None => (Some(name), true, None),
    }
}

/// Unwrap single-expression parentheses, ignoring comments but preserving sequences and empty wrappers.
pub fn unparenthesized(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" {
        let expressions: Vec<_> = named_children(node).into_iter().filter(|c| c.kind() != "comment").collect();
        if expressions.len() != 1 {
            break;
        }
        node = expressions[0];
    }
    node
}

/// Use callback values as carriers, naming them from supported bindings in matching source bytes.
/// Unwrap single-expression parentheses; keep other declarations as named nodes.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    let expression = unparenthesized(node);
    let mut node = if CALLBACK_TYPES.contains(&expression.kind()) { expression } else { node };
    let value_field = if node.kind() == "assignment_expression" { "right" } else { "value" };
    let value = node.child_by_field_name(value_field).map(unparenthesized);
    if matches!(node.kind(), "variable_declarator" | "assignment_expression" | "pair")
        && let Some(value) = value.filter(|n| CALLBACK_TYPES.contains(&n.kind()))
    {
        node = value;
    }
    if CALLBACK_TYPES.contains(&node.kind()) {
        callback_carrier(node, callback_binding(node, source), source)
    } else {
        named_carrier(node, source)
    }
}
