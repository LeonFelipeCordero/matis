use super::javascript;
use crate::{
    language_syntax::property_callable,
    parsing::syntax::{DeclarationCarrier, named_carrier},
};
use tree_sitter::Node;
pub const QUERY: &str = "\n[(function_declaration) (generator_function_declaration)] @definition.function\n[(class_declaration) (class)] @definition.class\n(method_definition) @definition.method\n(interface_body (property_signature name: (_) @name) @definition.method)\n[(arrow_function) (function_expression) (generator_function)] @definition.function\n";

/// Mark function-typed property signatures as methods; return `None` for other properties.
/// Delegate other nodes to JavaScript normalization using source bytes matching the node's tree.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> Option<DeclarationCarrier<'tree>> {
    if node.kind() != "property_signature" {
        return Some(javascript::declaration(node, source));
    }
    property_callable(node)?;
    let mut carrier = named_carrier(node, source);
    carrier.kind = Some("method".to_owned());
    Some(carrier)
}
