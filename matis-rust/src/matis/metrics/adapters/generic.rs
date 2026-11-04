//! Conservative generic resolution and shared grammar target helpers.
use super::resolution_policy::ResolutionAdapter;
use crate::metrics::resolution_syntax::Syntax;
use crate::syntax::{field, named_children};
use serde_json::{Value, json};
use tree_sitter::Node;

/// Return sorted policy-registration metadata for `languages`, marking entries in `dedicated`.
/// Report registration only; hooks are empty and grammar behavior is not verified here.
pub fn receiver_coverage(languages: &[&str], dedicated: &[&str]) -> Value {
    let mut languages = languages.to_vec();
    languages.sort_unstable();
    let mut result = serde_json::Map::new();
    for language in languages {
        let registered = dedicated.contains(&language);
        result.insert(language.into(), json!({
            "status": if registered { "dedicated" } else { "generic" },
            "hooks": [],
            "verification": "not_verified_here",
            "limitations": if registered { "Dedicated policy" } else { "Unresolved-only receiver fallback; no grammar hooks verified." },
        }));
    }
    Value::Object(result)
}

/// Remove single-child parenthesized-expression wrappers, preserving `None` and other shapes.
pub fn unwrapped(mut node: Option<Node<'_>>) -> Option<Node<'_>> {
    while let Some(current) = node {
        let children = named_children(current);
        if current.kind() != "parenthesized_expression" || children.len() != 1 {
            break;
        }
        node = Some(children[0]);
    }
    node
}

/// Return `(callee, receiver)` using shared fields and `adapter` member-node metadata.
/// Missing syntax yields `(None, None)`; missing callee fields fall back to the first named child.
pub fn default_call_target<'a>(
    adapter: &ResolutionAdapter,
    node: Option<Node<'a>>,
) -> (Option<Node<'a>>, Option<Node<'a>>) {
    let Some(node) = node else {
        return (None, None);
    };
    let mut callee = field(node, &["function", "callee", "target", "name", "method"]).or_else(|| node.named_child(0));
    let mut receiver = field(node, &["object", "receiver"]);
    let member = if member_fields(adapter, node.kind()).is_some() { Some(node) } else { callee };
    if let Some(member) = member
        && let Some((receiver_fields, callee_fields)) = member_fields(adapter, member.kind())
    {
        receiver = field(member, receiver_fields);
        callee = field(member, callee_fields);
    }
    (callee, receiver)
}

fn member_fields<'a>(adapter: &'a ResolutionAdapter, kind: &str) -> Option<(&'a [&'a str], &'a [&'a str])> {
    adapter.member_nodes().iter().find(|(node, _, _)| *node == kind).map(|(_, receiver, callee)| (*receiver, *callee))
}

/// Extract shared `(callee, receiver)` nodes after unwrapping call and receiver parentheses.
pub fn call_target<'a>(adapter: &ResolutionAdapter, node: Option<Node<'a>>) -> (Option<Node<'a>>, Option<Node<'a>>) {
    let (callee, receiver) = default_call_target(adapter, unwrapped(node));
    (callee, unwrapped(receiver))
}

/// Allow class-expression inference only if the callee's named member is not rebound in `syntax`.
pub fn receiver_class_allowed(syntax: &Syntax, callee: Option<Node<'_>>, member: &str) -> bool {
    !syntax.member_rebound(&callee.map(|n| syntax.text(n)).unwrap_or_default(), member)
}

/// Return an assignment/declaration/loop target according to `adapter`, retaining the input scope.
/// Unsupported nodes yield `(None, scope)`; the syntax argument is unused.
pub fn binding<'a>(
    adapter: &ResolutionAdapter,
    _syntax: &'a Syntax,
    (node, scope): (Node<'a>, Node<'a>),
) -> (Option<Node<'a>>, Node<'a>) {
    if adapter.assignments().contains(&node.kind()) {
        return (field(node, &["left", "name"]).or_else(|| node.named_child(0)), scope);
    }
    if adapter.declarations().contains(&node.kind()) {
        return (field(node, &["name", "declarator", "pattern"]), scope);
    }
    if adapter.loop_bindings().contains(&node.kind()) {
        return (field(node, &["left", "name"]), scope);
    }
    (None, scope)
}

/// Find a write operand by left/argument/operand field, then first named child, or return `None`.
pub fn write_target(node: Node<'_>) -> Option<Node<'_>> {
    field(node, &["left", "argument", "operand"]).or_else(|| node.named_child(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::walk;

    #[test]
    fn inventory_is_sorted_and_marks_registration_only() {
        let coverage = receiver_coverage(&["z", "go", "c"], &["go"]);
        assert_eq!(coverage.as_object().unwrap().keys().map(String::as_str).collect::<Vec<_>>(), ["c", "go", "z"]);
        assert_eq!(coverage["go"]["status"], "dedicated");
        assert_eq!(coverage["c"]["hooks"], json!([]));
    }

    #[test]
    fn unwraps_only_single_child_parentheses() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("javascript").unwrap()).unwrap();
        let tree = parser.parse(b"((x))();", None).unwrap();
        let outer = walk(tree.root_node()).find(|n| n.kind() == "parenthesized_expression").unwrap();
        assert_eq!(unwrapped(Some(outer)).unwrap().kind(), "identifier");
        assert!(unwrapped(None).is_none());
    }
}
