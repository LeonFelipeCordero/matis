use crate::{
    parsing::syntax::{
        self, CallSite, CallTarget, DeclarationCarrier, callback_binding, callback_carrier, named_carrier,
    },
    syntax::{children, named_children, text},
};
use tree_sitter::Node;
pub const QUERY: &str = "\n(package_header (identifier) @name) @definition.module\n(companion_object) @definition.class\n[(lambda_literal) (anonymous_function)] @definition.function\n";
pub const CALLBACK_TYPES: &[&str] = &["lambda_literal", "anonymous_function"];

fn identifier_name(node: Node<'_>, kind: &str, source: &[u8]) -> Option<String> {
    named_children(node).into_iter().find(|child| child.kind() == kind).map(|child| text(Some(child), source))
}

/// Normalize callbacks, function names, interfaces, and companion objects from matching source bytes.
/// Use `Companion` for a companion object without an explicit name.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    if CALLBACK_TYPES.contains(&node.kind()) {
        return callback_carrier(node, callback_binding(node, source), source);
    }
    let mut carrier = named_carrier(node, source);
    if node.kind() == "function_declaration" {
        carrier.name = identifier_name(node, "simple_identifier", source);
    }
    if node.kind() == "companion_object" && carrier.name.is_none() {
        carrier.name = Some(identifier_name(node, "type_identifier", source).unwrap_or_else(|| "Companion".to_owned()));
    }
    if node.kind() == "class_declaration" && children(node).iter().any(|c| c.kind() == "interface") {
        carrier.kind = Some("interface".to_owned());
    }
    carrier
}

fn member(target: Node<'_>, source: &[u8]) -> String {
    if target.kind() != "navigation_expression" {
        return syntax::call_member(target, source);
    }
    let suffix = named_children(target).into_iter().rev().find(|c| c.kind() == "navigation_suffix");
    let member = suffix.and_then(|suffix| suffix.named_child(0));
    text(Some(member.unwrap_or(target)), source).trim().to_owned()
}

/// Return the optional target name, bare-name flag, and qualifier from matching source bytes.
/// Use Kotlin navigation suffixes to extract member names.
pub fn normalize_call(site: &CallSite<'_>, source: &[u8]) -> CallTarget {
    syntax::call_target(site.node, source, member)
}
