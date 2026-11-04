//! Grammar-specific AST extraction from exact source bytes, without executing source.
use crate::domain::{models::SourceFile, mutation_candidates::*};
use crate::mutations::{canonical_json, digest};
use crate::syntax::{children, named_children, text};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashSet;
use tree_sitter::Node;

fn expressions(language: &str) -> &'static [&'static str] {
    match language {
        "python" => &[
            "comparison_operator",
            "boolean_operator",
            "binary_operator",
            "not_operator",
            "unary_operator",
            "augmented_assignment",
        ],
        "javascript" | "typescript" | "tsx" => {
            &["binary_expression", "unary_expression", "augmented_assignment_expression"]
        }
        "java" => &["binary_expression", "unary_expression", "assignment_expression"],
        "kotlin" => &[
            "comparison_expression",
            "equality_expression",
            "check_expression",
            "conjunction_expression",
            "disjunction_expression",
            "additive_expression",
            "multiplicative_expression",
            "prefix_expression",
            "assignment",
        ],
        "rust" => &["binary_expression", "unary_expression", "compound_assignment_expr"],
        "go" => &["binary_expression", "unary_expression", "assignment_statement"],
        "elixir" => &["binary_operator", "unary_operator"],
        _ => &[],
    }
}

fn conditions(language: &str) -> &'static [&'static str] {
    match language {
        "python" => &[
            "if_statement",
            "elif_clause",
            "while_statement",
            "conditional_expression",
            "if_clause",
            "assert_statement",
        ],
        "javascript" | "typescript" | "tsx" | "java" => {
            &["if_statement", "while_statement", "do_statement", "for_statement", "ternary_expression"]
        }
        "kotlin" => &["if_expression", "while_statement", "do_while_statement"],
        "rust" => &["if_expression", "while_expression"],
        "go" => &["if_statement", "for_clause", "for_statement"],
        _ => &[],
    }
}

fn callables(language: &str) -> &'static [&'static str] {
    match language {
        "python" => &["function_definition", "lambda"],
        "javascript" | "typescript" | "tsx" => &[
            "function_declaration",
            "function_expression",
            "generator_function_declaration",
            "generator_function",
            "arrow_function",
            "method_definition",
        ],
        "java" => &["method_declaration", "constructor_declaration", "lambda_expression"],
        "kotlin" => &["function_declaration", "anonymous_function", "lambda_literal"],
        "rust" => &["function_item", "closure_expression"],
        "go" => &["function_declaration", "method_declaration", "func_literal"],
        "elixir" => &["anonymous_function"],
        _ => &[],
    }
}

const EXCLUDED: &[&str] = &[
    "comment",
    "line_comment",
    "block_comment",
    "regex",
    "regex_pattern",
    "type",
    "type_annotation",
    "type_arguments",
    "type_parameters",
    "type_alias_declaration",
    "interface_declaration",
    "ambient_declaration",
    "conditional_type",
    "mapped_type_clause",
    "type_predicate",
    "type_query",
    "implements_clause",
    "extends_type_clause",
    "type_spec",
    "type_constraint",
    "type_constraints",
    "where_clause",
    "const_parameter",
    "array_type",
    "literal_pattern",
    "range_pattern",
    "let_condition",
    "let_chain",
    "token_tree",
    "macro_definition",
    "when_condition",
];

fn span(node: Node<'_>) -> MutationSpan {
    MutationSpan {
        start_byte: node.start_byte() as i64,
        end_byte: node.end_byte() as i64,
        start_line: node.start_position().row as i64 + 1,
        start_column: node.start_position().column as i64 + 1,
        end_line: node.end_position().row as i64 + 1,
        end_column: node.end_position().column as i64 + 1,
    }
}

fn unwrap(mut node: Node<'_>) -> Node<'_> {
    while matches!(node.kind(), "parenthesized_expression" | "parenthesized_expression_list")
        && node.named_child_count() == 1
    {
        node = node.named_child(0).unwrap();
    }
    node
}

fn operator_kind(operator: &str, language: &str, expression: &str) -> Option<MutationKind> {
    if operator == "*" && matches!(language, "rust" | "go") && expression == "unary_expression" {
        return None;
    }
    match operator {
        "==" | "!=" | "===" | "!==" | "<" | "<=" | ">" | ">=" | "in" | "not in" | "is" | "is not" | "instanceof"
        | "!in" | "!is" | "=~" => Some(MutationKind::Comparison),
        "and" | "or" | "not" | "&&" | "||" | "!" | "??" | "andalso" | "orelse" => Some(MutationKind::Logical),
        "+" | "-" | "*" | "/" | "//" | "%" | "**" => Some(MutationKind::Arithmetic),
        "+=" | "-=" | "*=" | "/=" | "//=" | "%=" | "**=" | "&&=" | "||=" | "??=" => Some(MutationKind::Assignment),
        _ => None,
    }
}

fn operators<'a>(node: Node<'a>, language: &str, source: &[u8]) -> Vec<(MutationKind, Node<'a>)> {
    if !expressions(language).contains(&node.kind()) {
        return vec![];
    }
    children(node)
        .into_iter()
        .enumerate()
        .filter_map(|(index, token)| {
            let field = node.field_name_for_child(index as u32);
            if token.is_named()
                && token.kind() != "assignment_operator"
                && !matches!(field, Some("operator" | "operators"))
            {
                return None;
            }
            let operator = text(Some(token), source).split_whitespace().collect::<Vec<_>>().join(" ");
            operator_kind(&operator, language, node.kind()).map(|kind| (kind, token))
        })
        .collect()
}

fn first_argument(node: Node<'_>) -> Option<Node<'_>> {
    named_children(node).into_iter().find(|child| child.kind() == "arguments")?.named_child(0)
}

fn elixir_target(node: Node<'_>, source: &[u8]) -> String {
    if node.kind() == "call" { text(node.child_by_field_name("target"), source) } else { String::new() }
}

fn elixir_guard(node: Node<'_>, source: &[u8]) -> bool {
    text(node.child_by_field_name("operator"), source) == "when"
}

fn elixir_condition<'a>(node: Node<'a>, source: &[u8]) -> Option<Node<'a>> {
    if node.kind() == "binary_operator" && elixir_guard(node, source) {
        return node.child_by_field_name("right");
    }
    if node.kind() != "call" {
        return None;
    }
    let target = node.child_by_field_name("target")?;
    if target.kind() != "identifier" || !matches!(text(Some(target), source).as_str(), "if" | "unless") {
        return None;
    }
    first_argument(node)
}

fn unfielded_condition<'a>(node: Node<'a>, language: &str) -> Option<Node<'a>> {
    let items: Vec<_> = named_children(node).into_iter().filter(|child| !child.kind().contains("comment")).collect();
    match (language, node.kind()) {
        ("python", "conditional_expression") if items.len() == 3 => Some(items[1]),
        ("python", "if_clause" | "assert_statement") => items.first().copied(),
        ("kotlin", "while_statement" | "do_while_statement") => {
            items.into_iter().find(|child| child.kind() != "control_structure_body")
        }
        ("go", "for_statement") => {
            items.into_iter().find(|child| !matches!(child.kind(), "block" | "for_clause" | "range_clause"))
        }
        _ => None,
    }
}

fn condition<'a>(node: Node<'a>, language: &str, source: &[u8]) -> Option<Node<'a>> {
    if language == "elixir" {
        return elixir_condition(node, source);
    }
    if !conditions(language).contains(&node.kind()) {
        return None;
    }
    if let Some(condition) = node.child_by_field_name("condition") {
        return (!matches!(condition.kind(), "empty_statement" | ";")).then(|| unwrap(condition));
    }
    unfielded_condition(node, language)
}

fn elixir_name(node: Node<'_>) -> Option<Node<'_>> {
    let mut head = first_argument(node)?;
    if head.kind() == "binary_operator" {
        head = head.child_by_field_name("left")?;
    }
    if head.kind() == "identifier" { Some(head) } else { head.child_by_field_name("target") }
}

fn assigned_name(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    if !matches!(parent.kind(), "variable_declarator" | "assignment" | "variable_declaration") {
        return None;
    }
    parent.child_by_field_name("name").or_else(|| parent.child_by_field_name("left"))
}

fn callable(node: Node<'_>, language: &str, source: &[u8]) -> Option<MutationCallable> {
    let definition = language == "elixir" && matches!(elixir_target(node, source).as_str(), "def" | "defp");
    if !callables(language).contains(&node.kind()) && !definition {
        return None;
    }
    let mut name = node.child_by_field_name("name");
    if name.is_none() && language == "kotlin" {
        name = named_children(node).into_iter().find(|child| child.kind() == "simple_identifier");
    }
    if definition {
        name = elixir_name(node);
    }
    let name = name.or_else(|| assigned_name(node));
    Some(MutationCallable {
        name: name.map(|name| text(Some(name), source)).unwrap_or_else(|| "<anonymous>".into()),
        span: span(node),
        original_text: text(Some(node), source),
    })
}

fn elixir_pattern(node: Node<'_>, source: &[u8]) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    if parent.kind() == "stab_clause" {
        return parent.child_by_field_name("left") == Some(node);
    }
    if parent.kind() == "binary_operator" {
        return elixir_guard(parent, source) && parent.child_by_field_name("left") == Some(node);
    }
    if node.kind() != "call" || parent.kind() != "arguments" {
        return false;
    }
    parent.parent().is_some_and(|parent| matches!(elixir_target(parent, source).as_str(), "def" | "defp"))
}

fn skip(node: Node<'_>, language: &str, source: &[u8]) -> bool {
    if language == "elixir" {
        let excluded = matches!(elixir_target(node, source).as_str(), "quote" | "defmacro" | "defmacrop")
            || (node.kind() == "unary_operator" && text(Some(node), source).starts_with('@'))
            || elixir_pattern(node, source);
        if excluded {
            return true;
        }
    }
    EXCLUDED.contains(&node.kind()) || node.kind().contains("comment")
}

struct Extraction<'a> {
    source: &'a SourceFile,
    hash: String,
    found: Vec<MutationCandidate>,
    seen: HashSet<String>,
}

impl Extraction<'_> {
    fn candidate(
        &self,
        site: (MutationKind, Node<'_>, Node<'_>),
        owner: Option<MutationCallable>,
    ) -> MutationCandidate {
        let (kind, token, expression) = site;
        let source = self.source;
        let hash = &self.hash;
        let region = span(token);
        let identity = json!([hash, source.path, source.language, kind, region]);
        let original = text(Some(token), &source.source);
        MutationCandidate {
            id: digest(canonical_json(&identity).as_bytes()),
            path: source.path.clone(),
            language: source.language.clone(),
            source_hash: hash.into(),
            operator: (kind != MutationKind::Condition)
                .then(|| original.split_whitespace().collect::<Vec<_>>().join(" ")),
            kind,
            span: region,
            expression_span: span(expression),
            original_text: original,
            expression_text: text(Some(expression), &source.source),
            callable: owner,
        }
    }

    fn record(&mut self, node: Node<'_>, owner: &Option<MutationCallable>) {
        let source = self.source;
        let mut sites = Vec::new();
        if let Some(condition) = condition(node, &source.language, &source.source)
            && !matches!(condition.kind(), "let_condition" | "let_chain")
        {
            sites.push((MutationKind::Condition, condition, condition));
        }
        sites.extend(
            operators(node, &source.language, &source.source).into_iter().map(|(kind, token)| (kind, token, node)),
        );
        for site in sites {
            let item = self.candidate(site, owner.clone());
            if self.seen.insert(item.id.clone()) {
                self.found.push(item);
            }
        }
    }
}

/// Extract stable mutation candidates as JSON values from exact source bytes without executing them.
/// Sort by byte span, kind, and ID; retain original text and the enclosing callable when known.
/// Unsupported grammars, non-UTF-8 bytes, parser failures, and syntax errors yield no candidates.
///
/// # Errors
/// Returns an error if a candidate cannot be serialized to JSON.
pub fn extract_candidates(source: &SourceFile) -> Result<Vec<Value>> {
    if std::str::from_utf8(&source.source).is_err() || expressions(&source.language).is_empty() {
        return Ok(vec![]);
    }
    let Ok(tree) = super::syntax::parse(&source.language, &source.source) else {
        return Ok(vec![]);
    };
    if tree.root_node().has_error() {
        return Ok(vec![]);
    }
    let mut pending = vec![(tree.root_node(), None)];
    let mut extraction = Extraction { source, hash: digest(&source.source), found: Vec::new(), seen: HashSet::new() };
    while let Some((node, owner)) = pending.pop() {
        if node.named_child_count() == 0 || skip(node, &source.language, &source.source) {
            continue;
        }
        let owner = callable(node, &source.language, &source.source).or(owner);
        extraction.record(node, &owner);
        pending.extend(named_children(node).into_iter().rev().map(|child| (child, owner.clone())));
    }
    extraction.found.sort_by(|a, b| {
        (a.span.start_byte, a.span.end_byte, a.kind.as_str(), &a.id).cmp(&(
            b.span.start_byte,
            b.span.end_byte,
            b.kind.as_str(),
            &b.id,
        ))
    });
    extraction.found.into_iter().map(|item| Ok(serde_json::to_value(item)?)).collect()
}
