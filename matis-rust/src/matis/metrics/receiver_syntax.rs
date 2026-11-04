//! Grammar-specific receiver declarations; unknown annotations are not types.
use regex::Regex;
use std::sync::LazyLock;
use tree_sitter::Node;

use super::resolution_syntax::Syntax;
use crate::syntax::{children, field, named_children, text};

#[derive(Debug, Clone)]
pub struct ReceiverBinding<'tree> {
    pub name: String,
    pub annotation: Option<String>,
    pub scope: Node<'tree>,
    pub node: Node<'tree>,
    pub inferred: bool,
    pub member: bool,
    pub static_: bool,
}

pub const CLASSES: &[&str] = &["class_definition", "class_declaration", "struct_item", "type_spec"];

/// Strip supported wrappers and generic arguments, rejecting unsupported type syntax.
pub fn type_name(node: Option<Node<'_>>, source: &[u8]) -> Option<String> {
    let node = node?;
    match node.kind() {
        "generic_type" | "generic_type_with_turbofish" => {
            return type_name(field(node, &["name", "type"]).or_else(|| child(node, &["type_identifier"])), source);
        }
        "type_annotation" | "type" | "pointer_type" | "reference_type" | "nullable_type" => {
            return wrapped_type_name(node, source);
        }
        _ => {}
    }
    let mut value = text(Some(node), source);
    if node.kind() == "user_type" {
        value = value.split('<').next().unwrap_or("").to_owned();
    }
    // Python's \w means Unicode letters and numbers plus underscore, not combining marks.
    static TYPE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\A[\p{L}\p{N}_$]+(?:(?:\.|::)[\p{L}\p{N}_$]+)*\z").unwrap());
    TYPE.is_match(&value).then_some(value)
}

fn wrapped_type_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let entries: Vec<_> = named_children(node).into_iter().filter(|c| c.kind() != "lifetime").collect();
    if entries.len() == 1 { type_name(Some(entries[0]), source) } else { None }
}

/// Return the first direct named child with a kind in `kinds`, or `None`.
pub fn child<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    named_children(node).into_iter().find(|c| kinds.contains(&c.kind()))
}

/// Return the nearest ancestor with a kind in `kinds`, excluding `node`, or `None`.
pub fn enclosing<'tree>(node: Node<'tree>, kinds: &[&str]) -> Option<Node<'tree>> {
    std::iter::successors(node.parent(), Node::parent).find(|parent| kinds.contains(&parent.kind()))
}

#[derive(Debug, Clone, Default)]
pub struct Declaration<'tree> {
    pub name: Option<Node<'tree>>,
    pub annotation: Option<String>,
    pub value: Option<Node<'tree>>,
    pub member: bool,
}

/// Build one receiver binding from `details`, or none if its name or scope is missing.
/// Members use an enclosing type scope; missing annotations and Java `var` permit
/// conservative constructor or injected-parameter inference using `syntax`.
pub fn declaration<'tree>(
    syntax: &'tree Syntax,
    node: Node<'tree>,
    details: Declaration<'tree>,
) -> Vec<ReceiverBinding<'tree>> {
    let scope = if details.member { enclosing(node, CLASSES) } else { Some(syntax.scope(node)) };
    let (Some(name), Some(scope)) = (details.name, scope) else {
        return vec![];
    };
    let annotation = details.annotation.as_ref().filter(|s| s.as_str() != "var").cloned();
    let inferred = annotation.is_none();
    let annotation = if inferred { inferred_annotation(syntax, node, &details) } else { annotation };
    vec![ReceiverBinding {
        name: syntax.text(name),
        annotation,
        scope,
        node,
        inferred,
        member: details.member,
        static_: static_field(node, &syntax.source),
    }]
}

fn inferred_annotation(syntax: &Syntax, node: Node<'_>, details: &Declaration<'_>) -> Option<String> {
    let mut value = details.value;
    if details.member && member_assignment(node) && !constructor(node, &syntax.source) {
        value = None;
    }
    let annotation = constructed_annotation(syntax, value);
    if details.member && value.is_some_and(|v| v.kind() == "identifier") {
        return injected_parameter(syntax, node, &text(value, &syntax.source));
    }
    annotation
}

fn constructed_annotation(syntax: &Syntax, value: Option<Node<'_>>) -> Option<String> {
    let value = value?;
    if !syntax.adapter.receiver_class_nodes().contains(value.kind()) {
        return None;
    }
    let (callee, receiver) = syntax.adapter.call_target(Some(value));
    if receiver.is_none() { type_name(callee, &syntax.source) } else { None }
}

/// Test whether the left-hand target is an attribute or member expression.
pub fn member_assignment(node: Node<'_>) -> bool {
    field(node, &["left"]).is_some_and(|target| matches!(target.kind(), "attribute" | "member_expression"))
}

/// Detect static tokens or modifiers on a declaration or its variable-declarator parent.
/// Read modifier spellings from the matching `source` bytes.
pub fn static_field(node: Node<'_>, source: &[u8]) -> bool {
    let owner = if node.kind() == "variable_declarator" { node.parent() } else { Some(node) };
    owner.is_some_and(|owner| {
        children(owner).into_iter().any(|c| {
            c.kind() == "static"
                || c.kind() == "modifiers" && text(Some(c), source).split_whitespace().any(|s| s == "static")
        })
    })
}

/// Test whether the nearest function or method is named `__init__` or `constructor`.
/// Read its name from matching `source` bytes.
pub fn constructor(node: Node<'_>, source: &[u8]) -> bool {
    enclosing(node, &["function_definition", "method_definition"])
        .is_some_and(|method| matches!(text(field(method, &["name"]), source).as_str(), "__init__" | "constructor"))
}

/// Return the unique enclosing parameter's annotation for `name`, or `None`.
/// Reject parameters reassigned anywhere in the enclosing method.
pub fn injected_parameter(syntax: &Syntax, node: Node<'_>, name: &str) -> Option<String> {
    let method = enclosing(node, &["function_definition", "method_definition"])?;
    let parameters = field(method, &["parameters"])?;
    if parameter_written(syntax, method, name) {
        return None;
    }
    let matches: Vec<_> = named_children(parameters)
        .into_iter()
        .flat_map(|parameter| syntax.adapter.receiver_bindings(syntax, parameter))
        .filter(|b| b.name == name)
        .collect();
    if matches.len() == 1 { matches[0].annotation.clone() } else { None }
}

fn parameter_written(syntax: &Syntax, method: Node<'_>, name: &str) -> bool {
    syntax.nodes().into_iter().any(|n| {
        syntax.adapter.assignments().contains(n.kind())
            && method.start_byte() <= n.start_byte()
            && n.start_byte() < method.end_byte()
            && text(field(n, &["left"]), &syntax.source) == name
    })
}

/// Extract Java parameter, local, or field receiver bindings from `node` in `syntax`.
/// Return no bindings for unsupported nodes.
pub fn java_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    if node.kind() == "formal_parameter" {
        return declaration(
            syntax,
            node,
            Declaration {
                name: field(node, &["name"]),
                annotation: type_name(field(node, &["type"]), &syntax.source),
                ..Default::default()
            },
        );
    }
    let Some(parent) = node.parent().filter(|_| node.kind() == "variable_declarator") else {
        return vec![];
    };
    declaration(
        syntax,
        node,
        Declaration {
            name: field(node, &["name"]),
            annotation: type_name(field(parent, &["type"]), &syntax.source),
            value: field(node, &["value"]),
            member: parent.kind() == "field_declaration",
        },
    )
}

/// Extract Kotlin parameter/property receiver bindings, including `val`/`var` class parameters.
/// Plain class parameters and unsupported nodes in `syntax` produce no bindings.
pub fn kotlin_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    if !matches!(node.kind(), "class_parameter" | "parameter" | "variable_declaration") {
        return vec![];
    }
    let name = child(node, &["simple_identifier"]);
    let annotation = type_name(child(node, &["user_type", "nullable_type", "function_type"]), &syntax.source);
    let member = node.kind() == "class_parameter"
        && children(node).into_iter().any(|c| matches!(c.kind(), "val" | "var" | "binding_pattern_kind"));
    if node.kind() == "class_parameter" && !member {
        return vec![];
    }
    let (value, member) = kotlin_property(node, member);
    declaration(syntax, node, Declaration { name, annotation, value, member })
}

fn kotlin_property(node: Node<'_>, member: bool) -> (Option<Node<'_>>, bool) {
    if let Some(parent) = node.parent().filter(|p| p.kind() == "property_declaration") {
        return (child(parent, &["call_expression"]), parent.parent().is_some_and(|p| p.kind() == "class_body"));
    }
    (None, member)
}

/// Extract Python typed parameters, simple assignments, and `self` or class field bindings.
/// Use `syntax` for annotations and inference; unsupported targets produce no bindings.
pub fn python_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    if matches!(node.kind(), "typed_parameter" | "typed_default_parameter") {
        return declaration(
            syntax,
            node,
            Declaration {
                name: child(node, &["identifier"]),
                annotation: type_name(field(node, &["type"]), &syntax.source),
                ..Default::default()
            },
        );
    }
    if node.kind() != "assignment" {
        return vec![];
    }
    let Some(mut name) = field(node, &["left"]) else {
        return vec![];
    };
    let mut member = name.kind() == "attribute" && text(field(name, &["object"]), &syntax.source) == "self";
    let target = if member { field(name, &["attribute"]) } else { Some(name) };
    if !member {
        if name.kind() != "identifier" {
            return vec![];
        }
        member = syntax.scope(node).kind() == "class_definition";
    }
    // Keep absent attribute fields absent, just as the Python declaration builder does.
    let Some(target) = target else {
        return vec![];
    };
    name = target;
    declaration(
        syntax,
        node,
        Declaration {
            name: Some(name),
            annotation: type_name(field(node, &["type"]), &syntax.source),
            value: field(node, &["right"]),
            member,
        },
    )
}

/// Extract JavaScript/TypeScript parameters, variables, fields, and `this` assignments.
/// Use `syntax` for type inference; unsupported nodes produce no bindings.
pub fn script_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    if node.kind() == "assignment_expression" {
        let target = field(node, &["left"]);
        if let Some(target) =
            target.filter(|t| t.kind() == "member_expression" && text(field(*t, &["object"]), &syntax.source) == "this")
        {
            return declaration(
                syntax,
                node,
                Declaration {
                    name: field(target, &["property"]),
                    value: field(node, &["right"]),
                    member: true,
                    ..Default::default()
                },
            );
        }
        return vec![];
    }
    if !matches!(
        node.kind(),
        "required_parameter"
            | "optional_parameter"
            | "variable_declarator"
            | "public_field_definition"
            | "field_definition"
    ) {
        return vec![];
    }
    let member = node.kind().contains("field_definition") || child(node, &["accessibility_modifier"]).is_some();
    declaration(
        syntax,
        node,
        Declaration {
            name: field(node, &["name", "pattern", "property"]),
            annotation: type_name(field(node, &["type"]), &syntax.source),
            value: field(node, &["value"]),
            member,
        },
    )
}

/// Extract Go parameter, variable, field, and short-declaration receiver bindings in `syntax`.
/// Pair short-declaration values only when counts match; unsupported nodes yield no bindings.
pub fn go_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    if node.kind() == "short_var_declaration" {
        return go_short_bindings(syntax, node);
    }
    if !matches!(node.kind(), "parameter_declaration" | "var_spec" | "field_declaration") {
        return vec![];
    }
    let mut cursor = node.walk();
    node.children_by_field_name("name", &mut cursor)
        .flat_map(|name| {
            declaration(
                syntax,
                node,
                Declaration {
                    name: Some(name),
                    annotation: type_name(field(node, &["type"]), &syntax.source),
                    member: node.kind() == "field_declaration",
                    ..Default::default()
                },
            )
        })
        .collect()
}

fn go_short_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    let names = field(node, &["left"]).map(named_children).unwrap_or_default();
    let values = field(node, &["right"]).map(named_children).unwrap_or_default();
    names
        .iter()
        .enumerate()
        .filter(|(_, name)| name.kind() == "identifier")
        .flat_map(|(index, name)| {
            declaration(
                syntax,
                node,
                Declaration {
                    name: Some(*name),
                    value: if values.len() == names.len() { Some(values[index]) } else { None },
                    ..Default::default()
                },
            )
        })
        .collect()
}

/// Extract Rust parameter, simple `let`, and field receiver bindings using `syntax`.
/// Destructured `let` targets and unsupported nodes produce no bindings.
pub fn rust_bindings<'tree>(syntax: &'tree Syntax, node: Node<'tree>) -> Vec<ReceiverBinding<'tree>> {
    if !matches!(node.kind(), "parameter" | "let_declaration" | "field_declaration") {
        return vec![];
    }
    let Some(name) = field(node, &["pattern", "name"]) else {
        return vec![];
    };
    if node.kind() == "let_declaration" && name.kind() != "identifier" {
        return vec![];
    }
    declaration(
        syntax,
        node,
        Declaration {
            name: Some(name),
            annotation: type_name(field(node, &["type"]), &syntax.source),
            value: field(node, &["value"]),
            member: node.kind() == "field_declaration",
        },
    )
}
