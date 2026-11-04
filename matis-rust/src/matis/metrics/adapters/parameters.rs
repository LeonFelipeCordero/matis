//! Shared grammar-backed parameter helpers; ambiguous headers stay unknown.
use crate::syntax::{field, named_children};
use tree_sitter::Node;

pub const COMMENTS: &[&str] = &["comment", "line_comment", "block_comment"];

/// Return named children of a parameter container in source order, excluding comments.
pub fn parameter_children(node: Node<'_>) -> Vec<Node<'_>> {
    named_children(node).into_iter().filter(|n| !COMMENTS.contains(&n.kind())).collect()
}

/// Find the parameters/parameter field, then the first named child whose kind is in `types`.
/// Return `None` if no supported parameter container exists.
pub fn parameter_node<'a>(node: Node<'a>, types: &[&str]) -> Option<Node<'a>> {
    field(node, &["parameters", "parameter"])
        .or_else(|| named_children(node).into_iter().find(|n| types.contains(&n.kind())))
}

/// Count named non-comment inputs except kinds in `excluded`.
/// Return `None` for a missing container or syntax errors, rather than guessing a count.
pub fn count_parameters(parameters: Option<Node<'_>>, excluded: &[&str]) -> Option<usize> {
    let parameters = parameters.filter(|n| !n.has_error())?;
    Some(parameter_children(parameters).iter().filter(|n| !excluded.contains(&n.kind())).count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::walk;

    #[test]
    fn parameter_count_excludes_comments_and_unknown_headers() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("java").unwrap()).unwrap();
        let tree = parser.parse(b"class A { void f(int a, /* skip */ int b) {} }", None).unwrap();
        let node = walk(tree.root_node()).find(|n| n.kind() == "method_declaration").unwrap();
        assert_eq!(count_parameters(parameter_node(node, &[]), &[]), Some(2));
        assert_eq!(count_parameters(None, &[]), None);
        let invalid = parser.parse(b"class A { void f(int a, int) {} }", None).unwrap();
        let parameters = walk(invalid.root_node()).find(|n| n.kind() == "formal_parameters").unwrap();
        assert_eq!(count_parameters(Some(parameters), &[]), None);
    }
}
