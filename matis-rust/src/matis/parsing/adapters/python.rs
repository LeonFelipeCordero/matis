use crate::{
    parsing::{
        queries,
        syntax::{DeclarationCarrier, callback_binding, callback_carrier, named_carrier},
    },
    syntax::text,
};
use tree_sitter::Node;
pub const CALLBACK_TYPES: &[&str] = &["lambda"];

/// Unwrap decorated definitions and retain their decorator text on the declaration carrier.
/// Mark lambdas as function carriers with supported bindings; `source` must match the node's tree.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    let node = if node.kind() == "decorated_definition" {
        node.child_by_field_name("definition").unwrap_or(node)
    } else {
        node
    };
    if node.kind() == "lambda" {
        return callback_carrier(node, callback_binding(node, source), source);
    }
    let mut carrier = named_carrier(node, source);
    if let Some(parent) = node.parent().filter(|p| p.kind() == "decorated_definition") {
        carrier.decorators = queries::selected_nodes(
            (parent, source),
            queries::metadata("python").expect("Python metadata query is bundled"),
            "metadata",
        )
        .into_iter()
        .filter(|c| c.parent() == Some(parent))
        .map(|c| text(Some(c), source))
        .collect();
    }
    carrier
}
