//! Small syntax-tree primitives shared by extraction and analysis.
use tree_sitter::Node;

/// Yield the root and its named descendants in depth-first source order.
pub fn walk(root: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    let mut pending = vec![root];
    std::iter::from_fn(move || {
        let node = pending.pop()?;
        let mut cursor = node.walk();
        pending.extend(node.named_children(&mut cursor).collect::<Vec<_>>().into_iter().rev());
        Some(node)
    })
}

/// Decode the node's byte range from `source` as UTF-8 with replacement.
/// Return an empty string for `None`; `source` must contain the node's range.
///
/// # Panics
/// Panics if the node's byte range is outside `source`.
pub fn text(node: Option<Node<'_>>, source: &[u8]) -> String {
    node.map(|node| String::from_utf8_lossy(&source[node.byte_range()]).into_owned()).unwrap_or_default()
}

/// Return the first available field, trying `names` in caller order.
/// Return `None` if no field exists.
pub fn field<'tree>(node: Node<'tree>, names: &[&str]) -> Option<Node<'tree>> {
    names.iter().find_map(|name| node.child_by_field_name(name))
}

/// Collect the node's direct named children in source order.
pub fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Collect all direct children in source order, including unnamed tokens.
pub fn children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}
