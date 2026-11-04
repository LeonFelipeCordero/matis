//! Native Kotlin metric and resolution hooks, extending Java where appropriate.
use super::parameters::{parameter_children, parameter_node};
use super::resolution::ImportBindings;
use super::resolution_paths::quoted_path;
use super::resolution_policy::ResolutionAdapter;
use super::{generic, java};
use crate::domain::models::Symbol;
use crate::metrics::resolution_contract::ImportRequest;
use crate::metrics::resolution_syntax::Syntax;
use crate::syntax::{children, named_children};
use regex::Regex;
use std::sync::LazyLock;
use tree_sitter::Node;

static IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\A|[^\p{L}\p{N}_])import\s+([\p{L}\p{N}_.]+)").unwrap());
static ALIAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\A|[^\p{L}\p{N}_])as\s*:?\s*([\p{L}\p{N}_]+)").unwrap());

/// Identify a Kotlin `if` inside the control-structure body of an enclosing `if` alternative.
pub fn alternative(node: Node<'_>) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    let Some(owner) = parent.parent() else {
        return false;
    };
    node.kind() == "if_expression"
        && parent.kind() == "control_structure_body"
        && owner.kind() == "if_expression"
        && owner.child_by_field_name("alternative") == Some(parent)
}

/// Count explicit Kotlin parameter declarations, counting destructuring as one input.
/// An explicit empty lambda header yields zero; implicit/error headers yield `None`.
/// The source argument is unused.
pub fn parameter_count(node: Node<'_>, _source: &[u8]) -> Option<usize> {
    let Some(parameters) = parameter_node(node, &["function_value_parameters", "lambda_parameters"]) else {
        return (node.kind() == "lambda_literal" && children(node).iter().any(|n| n.kind() == "->")).then_some(0);
    };
    if parameters.has_error() {
        return None;
    }
    Some(
        parameter_children(parameters)
            .iter()
            .filter(|n| ["parameter", "variable_declaration", "multi_variable_declaration"].contains(&n.kind()))
            .count(),
    )
}

/// Extract the last quoted path or Kotlin import name from `source`, with trimmed-source fallback.
pub fn import_path(source: &str) -> String {
    quoted_path(source)
        .unwrap_or_else(|| IMPORT.captures(source).map(|m| m[1].to_owned()).unwrap_or_else(|| source.trim().into()))
}

/// Find a Kotlin write operand, rejecting prefix/postfix expressions without `++` or `--`.
/// Other shapes use shared operand fields and first-child fallback.
pub fn write_target(node: Node<'_>) -> Option<Node<'_>> {
    if ["prefix_expression", "postfix_expression"].contains(&node.kind())
        && !children(node).iter().any(|n| ["++", "--"].contains(&n.kind()))
    {
        return None;
    }
    generic::write_target(node)
}

/// Return `(callee, receiver)` with Kotlin navigation suffixes separated after Java-style extraction.
/// Missing syntax retains empty target components.
pub fn call_target<'a>(adapter: &ResolutionAdapter, node: Option<Node<'a>>) -> (Option<Node<'a>>, Option<Node<'a>>) {
    let (mut callee, mut receiver) = java::call_target(adapter, node);
    if let Some(navigation) = callee.filter(|n| n.kind() == "navigation_expression") {
        let parts = named_children(navigation);
        receiver = parts.first().copied();
        callee = parts.last().map(|suffix| {
            let children = named_children(*suffix);
            children.last().copied().unwrap_or(*suffix)
        });
    }
    (callee, receiver)
}

/// Extract Kotlin property or loop-variable targets and scopes; loops use their own node as scope.
/// Other forms use Java binding rules.
pub fn binding<'a>(
    adapter: &ResolutionAdapter,
    syntax: &'a Syntax,
    (node, scope): (Node<'a>, Node<'a>),
) -> (Option<Node<'a>>, Node<'a>) {
    match node.kind() {
        "property_declaration" => {
            (named_children(node).into_iter().find(|n| n.kind() == "variable_declaration"), scope)
        }
        "for_statement" => (
            named_children(node)
                .into_iter()
                .find(|n| ["variable_declaration", "multi_variable_declaration"].contains(&n.kind())),
            node,
        ),
        _ => java::binding(adapter, syntax, (node, scope)),
    }
}

/// Copy the normalized Kotlin import `path`; the directive source is unused.
pub fn import_name(_source: &str, path: &str) -> String {
    path.into()
}

/// Return one `(alias or terminal name, declaration targets, false)` Kotlin import binding.
/// Preserve unresolved and ambiguous targets from the module index.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    let name = ALIAS
        .captures(&request.source)
        .map(|m| m[1].to_owned())
        .unwrap_or_else(|| request.path.rsplit('.').next().unwrap_or("").into());
    vec![(name, ctx.modules.declared(request.file, &request.path), false)]
}

/// Allow module and object members during static lookup without an explicit static modifier.
pub fn static_member(parent: &Symbol) -> bool {
    ["module", "object"].contains(&parent.kind.as_str())
}

// Kotlin inherits these package, overload and qualified-name hooks without changes.
pub use java::{inheritance, module_identity, package, qualify_class, same_package, select};

/// Return the unique declaration key for the Kotlin request path, or `None` if unresolved or ambiguous.
pub fn import_key(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Option<String> {
    let targets = ctx.modules.declared(request.file, &request.path);
    (targets.len() == 1).then(|| targets[0].key.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::walk;

    #[test]
    fn kotlin_parameters_preserve_unknown_implicit_inputs() {
        let source =
            b"fun f(a: Int, b: String = \"x\") {}\nval a = { x: Int, (y, z) -> x }\nval b = { -> 1 }\nval c = { it }\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("kotlin").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let function = walk(tree.root_node()).find(|n| n.kind() == "function_declaration").unwrap();
        assert_eq!(parameter_count(function, source), Some(2));
        let counts: Vec<_> = walk(tree.root_node())
            .filter(|n| n.kind() == "lambda_literal")
            .map(|n| parameter_count(n, source))
            .collect();
        assert_eq!(counts, [Some(2), Some(0), None]);
    }

    #[test]
    fn import_and_alias_words_match_python_unicode() {
        assert_eq!(import_path("\u{0301}import café.Type as Local"), "café.Type");
        assert_eq!(import_path("import cafe\u{0301}.Type"), "cafe");
        assert_eq!(import_path("éimport pkg.Type"), "éimport pkg.Type");
        let cases = [
            ("\u{0301}as Café", Some("Café")),
            ("\u{200c}as Local", Some("Local")),
            ("as A²", Some("A²")),
            ("as Cafe\u{0301}", Some("Cafe")),
            ("éas Local", None),
            ("²as Local", None),
            ("_as Local", None),
        ];
        for (source, expected) in cases {
            assert_eq!(ALIAS.captures(source).and_then(|m| m.get(1)).map(|m| m.as_str()), expected, "{source}");
        }
    }

    #[test]
    fn else_if_and_write_targets_follow_grammar() {
        let source = b"fun f(x: Int) { if (x > 0) {} else if (x < 0) {}\n!x\nx++\n}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("kotlin").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        assert_eq!(walk(tree.root_node()).filter(|n| alternative(*n)).count(), 1);
        let unary: Vec<_> = walk(tree.root_node())
            .filter(|n| ["prefix_expression", "postfix_expression"].contains(&n.kind()))
            .collect();
        assert_eq!(unary.iter().map(|n| write_target(*n).is_some()).collect::<Vec<_>>(), [false, true]);
    }
}
