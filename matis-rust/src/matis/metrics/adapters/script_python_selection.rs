//! Python overload and bound-instance selection, using only native syntax.
use crate::domain::models::Symbol;
use crate::metrics::resolution_syntax::Syntax;
use crate::syntax::{field, named_children, text};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Extract Python `(imported name, alias)` pairs in directive order using matching `source`.
/// Missing aliases are empty strings.
pub fn import_names(node: Node<'_>, source: &[u8]) -> Vec<(String, String)> {
    let mut cursor = node.walk();
    node.children_by_field_name("name", &mut cursor)
        .map(|child| (text(field(child, &["name"]).or(Some(child)), source), text(field(child, &["alias"]), source)))
        .collect()
}

fn decorators(node: Node<'_>, source: &[u8]) -> Vec<String> {
    named_children(node)
        .into_iter()
        .filter(|c| c.kind() == "decorator")
        .map(|c| text(Some(c), source).trim_start_matches('@').trim().to_owned())
        .collect()
}

fn overload_target(imported: Node<'_>, (name, local): (&str, &str), source: &[u8]) -> String {
    if imported.kind() == "import_statement" && name == "typing" {
        return format!("{local}.overload");
    }
    if imported.kind() == "import_from_statement"
        && text(field(imported, &["module_name"]), source) == "typing"
        && name == "overload"
    {
        return local.into();
    }
    String::new()
}

/// Recognize `symbol` as a typing overload through a unique earlier visible import or alias.
/// Reject shadowed providers and rebound `.overload` members; missing syntax returns false.
pub fn typing_overload(syntax: &Syntax, symbol: &Symbol) -> bool {
    let Some(node) = syntax.node(symbol.start_byte, symbol.end_byte) else {
        return false;
    };
    let Some(parent) = node.parent().filter(|n| n.kind() == "decorated_definition") else {
        return false;
    };
    let decorators = decorators(parent, &syntax.source);
    let mut providers: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for imported in syntax.nodes().into_iter().filter(|n| {
        ["import_statement", "import_from_statement"].contains(&n.kind())
            && syntax.visible(syntax.scope(*n), node.start_byte())
            && n.start_byte() < node.start_byte()
    }) {
        add_providers(&mut providers, imported, &syntax.source);
    }
    providers.iter().any(|(local, targets)| {
        targets.len() == 1
            && decorators.contains(&targets[0])
            && !syntax.shadowed(local, node.start_byte(), &[])
            && !syntax.member_rebound(local, "overload")
    })
}

fn add_providers(providers: &mut BTreeMap<String, Vec<String>>, imported: Node<'_>, source: &[u8]) {
    for (name, alias) in import_names(imported, source) {
        let local = if alias.is_empty() { name.clone() } else { alias };
        let target = overload_target(imported, (&name, &local), source);
        providers.entry(local).or_default().push(target);
    }
}

/// Return the final non-overload symbol when every preceding source-ordered symbol is a typing overload.
/// `ordered` must belong to `syntax`; invalid groups or empty input yield no targets.
pub fn overload_implementation(syntax: &Syntax, ordered: Vec<Symbol>) -> Vec<Symbol> {
    let Some(last) = ordered.last() else {
        return vec![];
    };
    if ordered[..ordered.len() - 1].iter().all(|s| typing_overload(syntax, s)) && !typing_overload(syntax, last) {
        vec![last.clone()]
    } else {
        vec![]
    }
}

fn descriptor_method(syntax: &Syntax, owner: Node<'_>) -> bool {
    let mut descriptors: BTreeSet<String> =
        ["staticmethod", "classmethod", "builtins.staticmethod", "builtins.classmethod"]
            .into_iter()
            .map(str::to_owned)
            .collect();
    descriptors.extend(
        syntax
            .nodes()
            .into_iter()
            .filter(|n| {
                n.kind() == "import_from_statement" && text(field(*n, &["module_name"]), &syntax.source) == "builtins"
            })
            .flat_map(|n| import_names(n, &syntax.source))
            .filter(|(name, _)| ["staticmethod", "classmethod"].contains(&name.as_str()))
            .map(|(name, alias)| if alias.is_empty() { name } else { alias }),
    );
    named_children(owner)
        .into_iter()
        .any(|c| descriptors.contains(text(Some(c), &syntax.source).trim_start_matches('@').trim()))
}

fn instance_owner(syntax: &Syntax, node: Node<'_>) -> bool {
    let mut owner = node.parent();
    if let Some(decorated) = owner.filter(|n| n.kind() == "decorated_definition") {
        if descriptor_method(syntax, decorated) {
            return false;
        }
        owner = decorated.parent();
    }
    owner.and_then(|n| n.parent()).is_some_and(|n| n.kind() == "class_definition")
}

/// Test whether the nearest enclosing Python function has an unwritten first `self` parameter.
/// Require an instance method, excluding static/class descriptors; missing syntax returns false.
pub fn bound_self(syntax: &Syntax, mut node: Option<Node<'_>>) -> bool {
    while node.is_some_and(|n| n.kind() != "function_definition") {
        node = node.and_then(|n| n.parent());
    }
    let Some(node) = node else {
        return false;
    };
    let first = field(node, &["parameters"]).and_then(|p| p.named_child(0));
    if first.is_none_or(|n| text(Some(n), &syntax.source).split(':').next() != Some("self"))
        || !instance_owner(syntax, node)
    {
        return false;
    }
    !syntax.nodes().into_iter().any(|n| {
        syntax.adapter.assignments().contains(&n.kind())
            && node.start_byte() <= n.start_byte()
            && n.start_byte() < node.end_byte()
            && text(field(n, &["left", "name"]), &syntax.source) == "self"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_aliases_preserve_order() {
        let source = b"from typing import overload as ov, Any\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("python").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(
            import_names(tree.root_node().named_child(0).unwrap(), source),
            vec![("overload".into(), "ov".into()), ("Any".into(), "".into())]
        );
    }
}
