//! Shared grammar-backed complexity and parameter policies.
use super::{elixir, go, java, javascript, kotlin, python, rust, typescript};
use crate::syntax::{children, named_children};
use tree_sitter::Node;

pub const LOOPS: &[&str] = &[
    "for_statement",
    "for_in_statement",
    "enhanced_for_statement",
    "foreach_statement",
    "for_each_statement",
    "while_statement",
    "do_statement",
    "do_while_statement",
    "for_expression",
    "while_expression",
    "loop_expression",
    "for_in_clause",
    "for_range_loop",
];
pub const DECISIONS: &[&str] = &[
    "for_statement",
    "for_in_statement",
    "enhanced_for_statement",
    "foreach_statement",
    "for_each_statement",
    "while_statement",
    "do_statement",
    "do_while_statement",
    "for_expression",
    "while_expression",
    "loop_expression",
    "for_in_clause",
    "for_range_loop",
    "if_statement",
    "if_expression",
    "elif_clause",
    "if_clause",
    "conditional_expression",
    "ternary_expression",
    "catch_clause",
    "except_clause",
    "switch_case",
    "case_statement",
    "switch_label",
    "expression_case",
    "type_case",
    "match_arm",
    "when_entry",
    "case_clause",
];
pub const SCOPES: &[&str] = &[
    "function_definition",
    "function_declaration",
    "function_expression",
    "generator_function",
    "generator_function_declaration",
    "arrow_function",
    "method_definition",
    "method_declaration",
    "constructor_declaration",
    "function_item",
    "lambda",
    "lambda_expression",
    "lambda_literal",
    "anonymous_function",
    "func_literal",
    "closure_expression",
    "class_definition",
    "class_declaration",
    "class",
    "object_declaration",
    "class_specifier",
];
pub const BOOLEANS: &[&str] = &[
    "binary_expression",
    "boolean_operator",
    "binary_operator",
    "infix_expression",
    "conjunction_expression",
    "disjunction_expression",
];

/// Disable an optional metric predicate by returning false for every node.
pub fn never(_node: Node<'_>) -> bool {
    false
}

/// Identify `elif` clauses and `if` nodes used as another conditional's alternative.
pub fn alternative(node: Node<'_>) -> bool {
    if node.kind() == "elif_clause" {
        return true;
    }
    if !["if_statement", "if_expression"].contains(&node.kind()) {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    if parent.kind() == "else_clause" {
        return conditional_parent(parent.parent());
    }
    conditional_parent(Some(parent)) && parent.child_by_field_name("alternative") == Some(node)
}

fn conditional_parent(node: Option<Node<'_>>) -> bool {
    node.is_some_and(|n| ["if_statement", "if_expression"].contains(&n.kind()))
}

#[derive(Clone, Debug)]
pub struct MetricPolicy {
    pub language: String,
    pub merged_defaults: bool,
}

pub const COMMON_METRICS: MetricPolicy = MetricPolicy { language: String::new(), merged_defaults: false };
pub const DEFAULT_METRICS: MetricPolicy = MetricPolicy { language: String::new(), merged_defaults: true };

impl MetricPolicy {
    /// Select dedicated grammar rules for `language`, or merged defaults for other languages.
    pub fn new(language: &str) -> Self {
        let dedicated = ["javascript", "jsx", "typescript", "tsx", "java", "kotlin", "go", "rust", "elixir", "python"];
        Self { language: language.into(), merged_defaults: !dedicated.contains(&language) }
    }

    /// Return a scope's body node, falling back to `root` when no supported body exists.
    pub fn body<'a>(&self, root: Node<'a>) -> Node<'a> {
        if !SCOPES.contains(&root.kind()) {
            return root;
        }
        root.child_by_field_name("body")
            .or_else(|| {
                named_children(root)
                    .into_iter()
                    .find(|n| (self.merged_defaults || self.language == "kotlin") && n.kind() == "function_body")
            })
            .unwrap_or(root)
    }

    /// Test whether `node` starts a nested metric scope, using `source` for Elixir calls.
    pub fn scope(&self, node: Node<'_>, source: &[u8]) -> bool {
        SCOPES.contains(&node.kind())
            || ((self.merged_defaults || self.language == "elixir") && elixir::scope(node, source))
    }

    /// Apply language decision overrides, then shared rules that exclude default/else branches.
    /// `source` must match the node's byte offsets.
    pub fn decision(&self, node: Node<'_>, source: &[u8]) -> bool {
        self.decision_override(node, source).unwrap_or_else(|| common_decision(node))
    }

    fn includes(&self, language: &str) -> bool {
        self.merged_defaults || self.language == language
    }

    fn decision_override(&self, node: Node<'_>, source: &[u8]) -> Option<bool> {
        match node.kind() {
            "case_clause" if self.includes("python") => Some(python::case_decision(node)),
            "match_arm" if self.includes("rust") => Some(rust::match_decision(node, source)),
            "call" if self.includes("elixir") => Some(elixir::call_decision(node, source)),
            "stab_clause" if self.includes("elixir") => Some(elixir::clause_decision(node, source)),
            "binary_operator" if self.includes("elixir") => Some(elixir::guard_decision(node, source)),
            _ => None,
        }
    }

    /// Test whether a supported boolean expression contains an and/or operator token.
    pub fn logical(&self, node: Node<'_>) -> bool {
        BOOLEANS.contains(&node.kind()) && children(node).iter().any(|n| ["&&", "||", "and", "or"].contains(&n.kind()))
    }

    /// Recognize shared or Kotlin-specific else-if syntax for decision-depth adjustment.
    pub fn alternative(&self, node: Node<'_>) -> bool {
        alternative(node) || ((self.merged_defaults || self.language == "kotlin") && kotlin::alternative(node))
    }

    /// Test whether the node kind contributes to loop nesting depth.
    pub fn is_loop(&self, node: Node<'_>) -> bool {
        LOOPS.contains(&node.kind())
    }
}

fn common_decision(node: Node<'_>) -> bool {
    if !DECISIONS.contains(&node.kind()) {
        return false;
    }
    !["switch_case", "case_statement", "switch_label", "when_entry"].contains(&node.kind())
        || !children(node).iter().any(|n| ["default", "else"].contains(&n.kind()))
}

impl Default for MetricPolicy {
    /// Return the merged default policy used when no language-specific policy is supplied.
    fn default() -> Self {
        DEFAULT_METRICS
    }
}

#[derive(Clone, Debug)]
pub struct MetricAdapter {
    pub language: String,
    pub metrics: MetricPolicy,
}

impl MetricAdapter {
    /// Pair `language` with its complexity policy and parameter-count dispatch.
    pub fn new(language: &str) -> Self {
        Self { language: language.into(), metrics: MetricPolicy::new(language) }
    }

    /// Count explicit callable inputs using matching `source` bytes and language rules.
    /// Return `None` for unsupported languages or unknown/invalid headers.
    pub fn parameter_count(&self, node: Node<'_>, source: &[u8]) -> Option<usize> {
        match self.language.as_str() {
            "javascript" | "jsx" => javascript::parameter_count(node, source),
            "typescript" | "tsx" => typescript::parameter_count(node, source),
            _ => self.non_script_parameter_count(node, source),
        }
    }

    fn non_script_parameter_count(&self, node: Node<'_>, source: &[u8]) -> Option<usize> {
        match self.language.as_str() {
            "java" => java::parameter_count(node, source),
            "kotlin" => kotlin::parameter_count(node, source),
            "go" => go::parameter_count(node, source),
            "python" => python::parameter_count(node, source),
            "rust" => rust::parameter_count(node, source),
            "elixir" => elixir::parameter_count(node, source),
            _ => None,
        }
    }
}

impl Default for MetricAdapter {
    /// Return an unnamed adapter with merged metrics and no parameter-count support.
    fn default() -> Self {
        Self::new("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::walk;

    #[test]
    fn parameter_dispatch_preserves_dedicated_aliases_and_unknowns() {
        let cases = [
            ("java", "class A { void f(int x) {} }", "method_declaration"),
            ("kotlin", "fun f(x: Int) {}", "function_declaration"),
            ("go", "package p\nfunc f(x int) {}", "function_declaration"),
            ("python", "def f(x): pass", "function_definition"),
            ("rust", "fn f(x: i32) {}", "function_item"),
            ("elixir", "def f(x), do: x", "call"),
            ("javascript", "function f(x) {}", "function_declaration"),
            ("jsx", "function f(x) {}", "function_declaration"),
            ("typescript", "function f(x: number) {}", "function_declaration"),
            ("tsx", "function f(x: number) {}", "function_declaration"),
        ];
        for (language, source, kind) in cases {
            let grammar = if language == "jsx" { "javascript" } else { language };
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&tree_sitter_language_pack::get_language(grammar).unwrap()).unwrap();
            let tree = parser.parse(source, None).unwrap();
            let node = walk(tree.root_node()).find(|n| n.kind() == kind).unwrap();
            assert_eq!(MetricAdapter::new(language).parameter_count(node, source.as_bytes()), Some(1), "{language}");
            assert_eq!(MetricAdapter::new("lua").parameter_count(node, source.as_bytes()), None);
        }
    }

    #[test]
    fn merged_default_and_dedicated_policies_differ() {
        let source = b"fn f(x: i32) { match x { _ => (), 1 => () } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("rust").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let arm = walk(tree.root_node()).find(|n| n.kind() == "match_arm").unwrap();
        assert!(!MetricPolicy::new("c").decision(arm, source));
        assert!(MetricPolicy::new("java").decision(arm, source));
        const {
            const POLICY: &MetricPolicy = &COMMON_METRICS;
            assert!(!POLICY.merged_defaults);
        };
    }

    #[test]
    fn merged_defaults_include_elixir_and_kotlin_hooks() {
        let source = b"defmodule Demo do\n def f(x), do: if(x, do: 1, else: 0)\nend";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("elixir").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let policy = MetricPolicy::new("lua");
        assert_eq!(walk(tree.root_node()).filter(|n| policy.scope(*n, source)).count(), 2);
        assert_eq!(walk(tree.root_node()).filter(|n| policy.decision(*n, source)).count(), 1);
        let source = b"fun f(x: Int) { if (x > 0) {} else if (x < 0) {} }";
        parser.set_language(&tree_sitter_language_pack::get_language("kotlin").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let root = walk(tree.root_node()).find(|n| n.kind() == "function_declaration").unwrap();
        assert_eq!(policy.body(root).kind(), "function_body");
        assert_eq!(walk(tree.root_node()).filter(|n| policy.alternative(*n)).count(), 1);
    }

    #[test]
    fn common_decisions_exclude_default_and_detect_logic() {
        let source =
            b"class A { void f(int x) { switch(x) { case 1: break; default: break; } if (x > 0 && x < 3) {} } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("java").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let policy = MetricPolicy::new("java");
        let labels: Vec<_> = walk(tree.root_node()).filter(|n| n.kind() == "switch_label").collect();
        assert_eq!(labels.iter().map(|n| policy.decision(*n, source)).collect::<Vec<_>>(), [true, false]);
        assert_eq!(walk(tree.root_node()).filter(|n| policy.logical(*n)).count(), 1);
    }
}
