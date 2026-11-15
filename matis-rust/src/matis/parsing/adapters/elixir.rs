use crate::{
    language_syntax::{DECLARATIONS, declaration_head},
    parsing::queries,
    parsing::syntax::{
        self, CallSite, CallTarget, CallVisitor, DeclarationCarrier, callback_binding, callback_carrier, named_carrier,
    },
    syntax::{field, text},
};
use tree_sitter::Node;
pub const CALLBACK_TYPES: &[&str] = &["anonymous_function"];
/// Mark anonymous functions as callbacks with supported bindings; keep other declaration names.
/// `source` must correspond to the node's syntax tree.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    if node.kind() == "anonymous_function" {
        callback_carrier(node, callback_binding(node, source), source)
    } else {
        named_carrier(node, source)
    }
}

fn target(node: Node<'_>, source: &[u8]) -> String {
    let target = if node.kind() == "call" { field(node, &["target"]) } else { None };
    text(target.filter(|n| n.kind() == "identifier"), source)
}

fn noncall(node: Node<'_>, source: &[u8]) -> bool {
    let mut parent = node.parent();
    while let Some(owner) = parent {
        if DECLARATIONS.contains(&target(owner, source).as_str()) {
            return declaration_head(owner)
                .is_some_and(|head| head.start_byte() <= node.start_byte() && node.end_byte() <= head.end_byte());
        }
        parent = owner.parent();
    }
    false
}

/// Return the optional target name, bare-name flag, and qualifier from matching source bytes.
/// Treat pipeline right-side identifiers as bare calls; use shared syntax for other sites.
pub fn normalize_call(site: &CallSite<'_>, source: &[u8]) -> CallTarget {
    if site.node.kind() == "identifier" {
        (Some(text(Some(site.node), source)), true, None)
    } else {
        syntax::normalize_call(site, source)
    }
}

/// Invoke `visitor` for calls and pipeline right-side identifiers in depth-first source order.
/// Exclude special forms and declaration heads; `source` must match the root's syntax tree.
pub fn visit_calls(root: Node<'_>, source: &[u8], visitor: &mut CallVisitor<'_>) {
    for node in queries::selected_nodes((root, source), queries::calls("elixir"), "site") {
        if node.kind() == "identifier" || !noncall(node, source) {
            visitor(CallSite::new(node), source);
        }
    }
}
