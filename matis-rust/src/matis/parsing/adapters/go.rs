use crate::{
    parsing::syntax::{DeclarationCarrier, callback_carrier, named_carrier},
    syntax::{named_children, text},
};
use tree_sitter::Node;
pub const QUERY: &str = "\n(func_literal) @definition.function\n";
pub const CALLBACK_TYPES: &[&str] = &["func_literal"];

fn binding(node: Node<'_>, source: &[u8]) -> Option<String> {
    let values = node.parent().filter(|p| p.kind() == "expression_list")?;
    let owner = values
        .parent()
        .filter(|p| matches!(p.kind(), "short_var_declaration" | "assignment_statement" | "var_spec"))?;
    let value_field = if owner.kind() == "var_spec" { "value" } else { "right" };
    if owner.child_by_field_name(value_field) != Some(values) {
        return None;
    }
    let index = named_children(values).iter().position(|child| *child == node)?;
    let mut cursor = owner.walk();
    let mut names: Vec<_> =
        if owner.kind() == "var_spec" { owner.children_by_field_name("name", &mut cursor).collect() } else { vec![] };
    if let Some(left) = owner.child_by_field_name("left") {
        names = named_children(left);
    }
    names.get(index).filter(|name| name.kind() == "identifier").map(|name| text(Some(*name), source))
}

/// Mark function literals as callbacks, using the identifier at their assignment/spec position if present.
/// Keep other declaration names; `source` must match the node's syntax tree.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    if node.kind() == "func_literal" {
        callback_carrier(node, binding(node, source), source)
    } else {
        named_carrier(node, source)
    }
}
