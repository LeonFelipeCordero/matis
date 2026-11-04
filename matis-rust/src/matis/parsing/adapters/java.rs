use crate::parsing::syntax::{DeclarationCarrier, callback_binding, callback_carrier, named_carrier};
use tree_sitter::Node;
pub const QUERY: &str =
    "\n(lambda_expression) @definition.function\n(package_declaration (_) @name) @definition.module\n";
pub const CALLBACK_TYPES: &[&str] = &["lambda_expression"];

/// Mark constructors with an unset carrier name and normalize lambda callbacks with supported bindings.
/// Keep other declaration names; `source` must match the node's syntax tree.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    if matches!(node.kind(), "constructor_declaration" | "compact_constructor_declaration") {
        return DeclarationCarrier { node, name: None, kind: Some("constructor".to_owned()), decorators: vec![] };
    }
    if node.kind() == "lambda_expression" {
        callback_carrier(node, callback_binding(node, source), source)
    } else {
        named_carrier(node, source)
    }
}
