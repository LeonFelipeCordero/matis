//! Call-site spellings for source policy, independent of graph target inference.
use super::{
    adapters::LanguageAdapter,
    queries,
    syntax::{CallSite, ParsedSyntax, callee_node},
};
use crate::syntax::{children, field, named_children, text, walk};
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};
use tree_sitter::Node;

#[cfg(test)]
#[path = "../../../tests/unit/parsing/actions_tests.rs"]
mod tests;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionKind {
    Call,
    Constructor,
}

pub(crate) struct Action {
    pub name: String,
    pub qualified: Option<String>,
    pub kind: ActionKind,
    pub line: usize,
}

pub(crate) fn visit(syntax: &ParsedSyntax<'_>, mut observe: impl FnMut(Action)) {
    let source = &syntax.source.source;
    let adapter = LanguageAdapter { language: &syntax.source.language };
    let java = (adapter.language == "java").then(|| java_imports(syntax.root, source));
    let noncalls: HashSet<_> = if adapter.language == "elixir" {
        queries::selected_nodes((syntax.root, source), include_str!("queries/actions/elixir-noncalls.scm"), "noncall")
            .into_iter()
            .collect()
    } else {
        HashSet::new()
    };
    let newlines: Vec<_> = source.iter().enumerate().filter_map(|(i, byte)| (*byte == b'\n').then_some(i)).collect();
    adapter.visit_calls(syntax.root, source, &mut |site, bytes| {
        if adapter.call_relationship(&site) != "CALLS"
            || site.node.has_error()
            || !noncalls.is_empty()
                && std::iter::successors(Some(site.node), Node::parent).any(|node| noncalls.contains(&node))
        {
            return;
        }
        let Some(name) = spelling(&site, bytes) else { return };
        let kind = match site.node.kind() {
            "new_expression" | "object_creation_expression" => ActionKind::Constructor,
            _ => ActionKind::Call,
        };
        let qualified = java.as_ref().and_then(|imports| java_qualified(site.node, &name, imports));
        let offset = (site.node.start_byte() as isize + site.offset) as usize;
        observe(Action { name, qualified, kind, line: newlines.partition_point(|i| *i < offset) + 1 });
    });
}

fn spelling(site: &CallSite<'_>, source: &[u8]) -> Option<String> {
    let node = site.node;
    match node.kind() {
        "identifier" => return Some(text(Some(node), source)), // Elixir pipeline shorthand.
        "macro_invocation" => return simple_name(field(node, &["macro"])?, source).map(|name| format!("{name}!")),
        _ => {}
    }
    let target = callee_node(node)?;
    let name = simple_name(target, source)?;
    if let Some(receiver) = field(node, &["object", "receiver"]) {
        return Some(format!("{}.{name}", simple_name(receiver, source)?));
    }
    Some(name)
}

fn simple_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier"
        | "simple_identifier"
        | "type_identifier"
        | "field_identifier"
        | "property_identifier"
        | "package_identifier"
        | "alias"
        | "self"
        | "this"
        | "super"
        | "crate" => Some(text(Some(node), source)),
        "parenthesized_expression" | "generic_type" => {
            simple_name(named_children(node).into_iter().find(|n| !is_comment(*n))?, source)
        }
        "member_expression"
        | "attribute"
        | "selector_expression"
        | "field_access"
        | "field_expression"
        | "dot"
        | "scoped_identifier"
        | "scoped_type_identifier"
        | "navigation_expression"
        | "navigation_suffix"
        | "qualified_identifier" => name_parts(node, source),
        _ => None,
    }
}

fn is_comment(node: Node<'_>) -> bool {
    node.kind().ends_with("comment")
}

fn name_parts(node: Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() == "member_expression" {
        return Some(format!(
            "{}.{}",
            simple_name(field(node, &["object"])?, source)?,
            simple_name(field(node, &["property"])?, source)?
        ));
    }
    let mut name = String::new();
    for part in children(node).into_iter().filter(|n| !is_comment(*n)) {
        match part.kind() {
            "." | "::" => name.push_str(part.kind()),
            "?." => name.push('.'),
            "type_arguments" => {}
            _ => name.push_str(&simple_name(part, source)?),
        }
    }
    (!name.is_empty()).then_some(name)
}

#[derive(Default)]
struct JavaImports {
    types: HashMap<String, String>,
    shadows: HashMap<String, Vec<JavaShadow>>,
}

struct JavaShadow {
    range: Range<usize>,
    value: bool,
}

fn java_imports(root: Node<'_>, source: &[u8]) -> JavaImports {
    let mut imports = JavaImports::default();
    for node in named_children(root).into_iter().filter(|n| n.kind() == "import_declaration") {
        let parts = children(node);
        if parts.iter().any(|n| matches!(n.kind(), "asterisk" | "*" | "static")) {
            continue;
        }
        let path = parts.iter().find(|n| n.kind() == "scoped_identifier").and_then(|node| simple_name(*node, source));
        let Some(path) = path else { continue };
        let Some((_, leaf)) = path.rsplit_once('.') else { continue };
        imports.types.insert(leaf.to_owned(), path);
    }
    if !imports.types.is_empty() {
        add_java_shadows(root, source, &mut imports);
        add_java_receiver_shadows(root, source, &mut imports);
    }
    imports
}

fn add_java_shadows(root: Node<'_>, source: &[u8], imports: &mut JavaImports) {
    for node in walk(root) {
        let Some((declaration, value)) = java_declaration(node) else { continue };
        let name = text(Some(declaration), source);
        if imports.types.contains_key(&name) {
            let range = java_scope(node, declaration);
            imports.shadows.entry(name).or_default().push(JavaShadow { range, value });
        }
    }
}

fn java_declaration(node: Node<'_>) -> Option<(Node<'_>, bool)> {
    let value = match node.kind() {
        "class_declaration"
        | "interface_declaration"
        | "enum_declaration"
        | "record_declaration"
        | "annotation_type_declaration" => false,
        "variable_declarator"
        | "formal_parameter"
        | "catch_formal_parameter"
        | "enhanced_for_statement"
        | "resource" => true,
        "identifier" if inferred_java_parameter(node) => return Some((node, true)),
        _ => return None,
    };
    Some((field(node, &["name"])?, value))
}

fn inferred_java_parameter(node: Node<'_>) -> bool {
    node.parent().is_some_and(|parent| {
        parent.kind() == "inferred_parameters"
            || parent.kind() == "lambda_expression" && field(parent, &["parameters"]) == Some(node)
    })
}

fn java_scope(node: Node<'_>, declaration: Node<'_>) -> Range<usize> {
    if node.kind() == "enhanced_for_statement"
        && let Some(body) = field(node, &["body"])
    {
        return body.byte_range();
    }
    let Some(scope) = std::iter::successors(node.parent(), Node::parent).find(|parent| {
        matches!(
            parent.kind(),
            "block"
                | "constructor_body"
                | "switch_block"
                | "for_statement"
                | "try_with_resources_statement"
                | "lambda_expression"
                | "catch_clause"
                | "method_declaration"
                | "constructor_declaration"
                | "record_declaration"
                | "class_body"
                | "interface_body"
                | "enum_body"
                | "annotation_type_body"
                | "program"
        )
    }) else {
        return node.byte_range();
    };
    if scope.kind() == "try_with_resources_statement"
        && let Some(body) = field(scope, &["body"])
    {
        return declaration.start_byte()..body.end_byte();
    }
    let start = match scope.kind() {
        "block" | "constructor_body" | "switch_block" | "for_statement" | "try_with_resources_statement" => {
            declaration.start_byte()
        }
        _ => scope.start_byte(),
    };
    start..scope.end_byte()
}

// shortcut: explicit Java imports only; use a Java symbol resolver for inferred receiver types and wildcard imports.
fn java_qualified(node: Node<'_>, name: &str, imports: &JavaImports) -> Option<String> {
    let constructor = is_constructor(node);
    let (head, tail) = name.split_once('.').map_or((name, ""), |(head, tail)| (head, tail));
    if !constructor && tail.is_empty() {
        return None;
    }
    if imports.shadows.get(head).is_some_and(|shadows| {
        shadows.iter().any(|shadow| (!constructor || !shadow.value) && shadow.range.contains(&node.start_byte()))
    }) {
        return None;
    }
    // `outer.new Inner()` names a member of the outer expression, not an imported type.
    if constructor
        && !children(node).into_iter().find(|part| !is_comment(*part)).is_some_and(|part| part.kind() == "new")
    {
        return None;
    }
    let imported = imports.types.get(head)?;
    Some(if tail.is_empty() { imported.clone() } else { format!("{imported}.{tail}") })
}

fn add_java_receiver_shadows(root: Node<'_>, source: &[u8], imports: &mut JavaImports) {
    let query = include_str!("queries/action_shadows/java.scm");
    for binding in queries::selected_nodes((root, source), query, "binding") {
        let name = text(Some(binding), source);
        if !imports.types.contains_key(&name) {
            continue;
        }
        let Some(container) = binding.parent().and_then(|node| node.parent()) else { continue };
        let scope = match container.kind() {
            "enum_body" => Some(container),
            _ => container.parent().and_then(|statement| field(statement, &["consequence"])),
        };
        if let Some(scope) = scope {
            imports.shadows.entry(name).or_default().push(JavaShadow { range: scope.byte_range(), value: true });
        }
    }
}
