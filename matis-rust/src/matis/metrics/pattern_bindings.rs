//! Pattern captures and the syntax regions in which they bind names.
use super::{Syntax, children, field, names};
use std::collections::BTreeSet;
use tree_sitter::Node;

type ScopedBindings<'a> = Vec<(String, Node<'a>, Node<'a>)>;

/// Return `(name, scope, declaration)` bindings for Python cases and Rust scoped patterns.
/// `None` delegates to ordinary binding extraction; `Some([])` is a recognized pattern without captures.
pub fn scoped<'a>(syntax: &'a Syntax, node: Node<'a>) -> Option<ScopedBindings<'a>> {
    if syntax.language == "python" && node.kind() == "case_clause" {
        let captures = children(node)
            .into_iter()
            .filter(|child| child.kind() == "case_pattern")
            .flat_map(|child| python_names(syntax, child));
        return Some(captures.map(|name| (name, syntax.scope(node), node)).collect());
    }
    if syntax.language != "rust" {
        return None;
    }
    let scopes = match node.kind() {
        "for_expression" => field(node, &["body"]).into_iter().collect(),
        "match_arm" => vec![node],
        "let_condition" => condition_scopes(node),
        _ => return None,
    };
    let captures = rust_names(field(node, &["pattern"]), &syntax.source);
    Some(scopes.into_iter().flat_map(|scope| captures.iter().map(move |name| (name.clone(), scope, node))).collect())
}

fn condition_scopes(node: Node<'_>) -> Vec<Node<'_>> {
    let mut scopes = Vec::new();
    let mut child = node;
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if candidate.kind() == "let_chain" {
            scopes.extend(children(candidate).into_iter().filter(|n| n.start_byte() >= child.end_byte()));
        } else if ["if_expression", "while_expression"].contains(&candidate.kind()) {
            scopes.extend(field(candidate, &["consequence", "body"]));
            break;
        } else {
            break;
        }
        child = candidate;
        parent = candidate.parent();
    }
    scopes
}

/// Extract Rust capture names from `node` and matching source, excluding type paths and guards.
/// Missing nodes and nonbinding pattern parts produce no names.
pub fn rust_names(node: Option<Node<'_>>, source: &[u8]) -> BTreeSet<String> {
    let Some(node) = node else { return BTreeSet::new() };
    match node.kind() {
        "scoped_identifier" | "scoped_type_identifier" | "type_identifier" | "remaining_field_pattern" => {
            BTreeSet::new()
        }
        "shorthand_field_identifier" => [String::from_utf8_lossy(&source[node.byte_range()]).into_owned()].into(),
        "field_pattern" | "parameter" => rust_names(field(node, &["pattern", "name"]), source),
        "tuple_struct_pattern" | "struct_pattern" => children(node)
            .into_iter()
            .filter(|child| Some(*child) != field(node, &["type"]))
            .flat_map(|child| rust_names(Some(child), source))
            .collect(),
        "match_pattern" => children(node)
            .into_iter()
            .filter(|child| Some(*child) != field(node, &["condition"]))
            .flat_map(|child| rust_names(Some(child), source))
            .collect(),
        _ if node.kind().contains("pattern") => {
            children(node).into_iter().flat_map(|child| rust_names(Some(child), source)).collect()
        }
        _ => names(Some(node), source),
    }
}

fn python_names(syntax: &Syntax, node: Node<'_>) -> BTreeSet<String> {
    let children = children(node);
    match node.kind() {
        "identifier" => [syntax.text(node)].into_iter().filter(|name| name != "_").collect(),
        "dotted_name" if children.len() == 1 => python_names(syntax, children[0]),
        "class_pattern" => children.into_iter().skip(1).flat_map(|child| python_names(syntax, child)).collect(),
        "keyword_pattern" => children.into_iter().skip(1).flat_map(|child| python_names(syntax, child)).collect(),
        "dict_pattern" => {
            let mut cursor = node.walk();
            node.children_by_field_name("value", &mut cursor)
                .chain(children.into_iter().filter(|child| child.kind() == "splat_pattern"))
                .flat_map(|child| python_names(syntax, child))
                .collect()
        }
        "case_pattern" | "as_pattern" | "union_pattern" | "list_pattern" | "tuple_pattern" | "splat_pattern" => {
            children.into_iter().flat_map(|child| python_names(syntax, child)).collect()
        }
        _ => BTreeSet::new(),
    }
}
