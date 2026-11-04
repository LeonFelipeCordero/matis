//! Native Go metric and package resolution hooks.
use super::generic;
use super::parameters::{parameter_children, parameter_node};
use super::resolution::{ImportBindings, Modules};
use super::resolution_paths::{quoted_path, top_symbols};
use super::resolution_policy::ResolutionAdapter;
use crate::domain::models::{ParsedFile, ParsedRepository, Symbol};
use crate::metrics::receiver_syntax::type_name;
use crate::metrics::resolution_contract::ImportRequest;
use crate::metrics::resolution_syntax::Syntax;
use crate::metrics::resolver::CallResolver;
use crate::syntax::{children, field};
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;
use tree_sitter::Node;

static DOT_IMPORT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"^\s*\.\s+['\"]"#).unwrap());
static IMPORT_ALIAS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*([A-Za-z_$][\p{L}\p{N}_$]*)\s+['\"]"#).unwrap());

/// Count Go inputs, expanding grouped names and counting each unnamed/variadic declaration once.
/// Missing, invalid, or unsupported headers yield `None`; the source argument is unused.
pub fn parameter_count(node: Node<'_>, _source: &[u8]) -> Option<usize> {
    let parameters = parameter_node(node, &[]).filter(|n| !n.has_error())?;
    let entries = parameter_children(parameters);
    if entries.iter().any(|n| !["parameter_declaration", "variadic_parameter_declaration"].contains(&n.kind())) {
        return None;
    }
    Some(
        entries
            .into_iter()
            .map(|n| {
                let mut cursor = n.walk();
                n.children_by_field_name("name", &mut cursor).count().max(1)
            })
            .sum(),
    )
}

/// Return `(repository-relative directory, package-clause text)` as a Go package identity.
/// Missing syntax or a missing clause uses empty clause text.
pub fn package(syntax: &BTreeMap<String, Syntax>, file: &ParsedFile) -> (String, String) {
    let directory = file.source.path.rsplit_once('/').map(|(parent, _)| parent).unwrap_or("").to_owned();
    let clause = syntax
        .get(&file.source.path)
        .and_then(|syntax| syntax.nodes().into_iter().find(|n| n.kind() == "package_clause").map(|n| syntax.text(n)))
        .unwrap_or_default();
    (directory, clause)
}

/// Return retained Go files with the same directory and package clause as `source`.
pub fn package_files(
    repository: &ParsedRepository,
    syntax: &BTreeMap<String, Syntax>,
    source: &ParsedFile,
) -> Vec<ParsedFile> {
    let identity = package(syntax, source);
    repository
        .files
        .iter()
        .filter(|file| file.source.language == "go" && package(syntax, file) == identity)
        .cloned()
        .collect()
}

/// Collect all symbols from files in `source`'s Go package, preserving file and symbol order.
pub fn package_symbols(resolver: &CallResolver, source: &ParsedFile) -> Vec<Symbol> {
    package_files(&resolver.repository, &resolver.syntax, source).into_iter().flat_map(|file| file.symbols).collect()
}

/// Test whether `symbol` is a Go method whose normalized receiver type name matches `parent`.
/// Missing syntax and non-method symbols return false.
pub fn receiver_method(resolver: &CallResolver, symbol: &Symbol, parent: &Symbol) -> bool {
    if symbol.kind != "method" {
        return false;
    }
    let Some(syntax) = resolver.syntax.get(&symbol.file_path) else {
        return false;
    };
    let parameter = syntax
        .node(symbol.start_byte, symbol.end_byte)
        .and_then(|n| field(n, &["receiver"]))
        .and_then(|n| n.named_child(0));
    let annotation = parameter.and_then(|n| type_name(field(n, &["type"]), &syntax.source));
    annotation.as_deref() == Some(parent.name.as_str())
}

/// Find named methods in the type's package whose receiver annotation names `parent`.
/// Return no candidates if the parent file is absent.
pub fn instance_members(resolver: &CallResolver, parent: &Symbol, name: &str) -> Vec<Symbol> {
    let Some(source) = resolver.files.get(&parent.file_path) else {
        return vec![];
    };
    package_symbols(resolver, source)
        .into_iter()
        .filter(|symbol| symbol.name == name && receiver_method(resolver, symbol, parent))
        .collect()
}

/// Return Go type declarations named `name` in `source`'s package, retaining ambiguous matches.
pub fn same_package(modules: &Modules, source: &ParsedFile, name: &str) -> Vec<Symbol> {
    package_files(&modules.repository, &modules.syntax, source)
        .into_iter()
        .flat_map(|file| file.symbols)
        .filter(|s| s.name == name && s.kind == "type")
        .collect()
}

/// Extract the last quoted Go import path, or trimmed `source` when none exists.
pub fn import_path(source: &str) -> String {
    quoted_path(source).unwrap_or_else(|| source.trim().into())
}

/// Return a Go `(local name, top-level targets, namespace flag)` import binding.
/// Reject dot imports, honor explicit aliases, and retain unresolved names with empty targets.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    if DOT_IMPORT.is_match(&request.source) {
        return vec![];
    }
    let alias = IMPORT_ALIAS
        .captures(&request.source)
        .map(|m| m[1].to_owned())
        .filter(|name| !["import", "require"].contains(&name.as_str()));
    let binding = alias.unwrap_or_else(|| {
        request.path.replace('.', "/").trim_end_matches('/').rsplit('/').next().unwrap_or("").into()
    });
    if binding.is_empty() {
        return vec![];
    }
    let targets = ctx.path_files(request.file, &request.path);
    let symbols = targets.iter().flat_map(top_symbols).collect();
    vec![(binding, symbols, true)]
}

/// Extract Go binding targets and scopes, keeping `:= range` declarations in the loop scope.
/// Plain range assignments use the enclosing scope; other forms use shared extraction.
pub fn binding<'a>(
    adapter: &ResolutionAdapter,
    syntax: &'a Syntax,
    (node, mut scope): (Node<'a>, Node<'a>),
) -> (Option<Node<'a>>, Node<'a>) {
    if node.kind() == "range_clause" {
        if !children(node).iter().any(|n| n.kind() == ":=")
            && let Some(parent) = node.parent()
        {
            scope = syntax.scope(parent);
        }
        return (field(node, &["left"]), scope);
    }
    generic::binding(adapter, syntax, (node, scope))
}

/// Return `(callee, receiver)` for Go calls, composite literals, and address-of composite literals.
/// Reject other unary receivers; use shared extraction for remaining forms.
pub fn call_target<'a>(adapter: &ResolutionAdapter, node: Option<Node<'a>>) -> (Option<Node<'a>>, Option<Node<'a>>) {
    if let Some(node) = node {
        if node.kind() == "unary_expression" {
            let operand = field(node, &["operand"]);
            if field(node, &["operator"]).is_some_and(|n| n.kind() == "&")
                && operand.is_some_and(|n| n.kind() == "composite_literal")
            {
                return (operand.and_then(|n| field(n, &["type"])), None);
            }
            return (None, None);
        }
        if node.kind() == "composite_literal" {
            return (field(node, &["type"]), None);
        }
    }
    generic::default_call_target(adapter, node)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::walk;

    fn parsed(path: &str, source: &str) -> ParsedFile {
        crate::parsing::parser::parse_file(crate::domain::models::SourceFile {
            path: path.into(),
            absolute_path: std::path::PathBuf::from(path),
            language: "go".into(),
            source: source.as_bytes().to_vec(),
        })
        .unwrap()
    }

    #[test]
    fn package_lookup_requires_directory_clause_and_language() {
        let files = vec![
            parsed("pkg/a.go", "package p\ntype Thing struct {}"),
            parsed("pkg/b.go", "package p\ntype Thing struct {}"),
            parsed("pkg/c.go", "package other\ntype Thing struct {}"),
            parsed("other/d.go", "package p\ntype Thing struct {}"),
        ];
        let repository = ParsedRepository { files };
        let syntax = repository
            .files
            .iter()
            .map(|f| (f.source.path.clone(), Syntax::new(f, ResolutionAdapter { language: "go".into() })))
            .collect();
        let modules = Modules::new(&repository, &syntax);
        let source = &repository.files[0];
        assert_eq!(package_files(&repository, &syntax, source).len(), 2);
        assert_eq!(same_package(&modules, source, "Thing").len(), 2);
        let bindings = ImportBindings::new(&repository, &syntax, &modules);
        let request = ImportRequest {
            file: source,
            path: "missing/pkg".into(),
            source: "alias \"missing/pkg\"".into(),
            position: None,
        };
        let entries = import_item(&bindings, &request);
        assert_eq!(entries[0].0, "alias");
        assert!(entries[0].1.is_empty());
        assert!(entries[0].2);
        let request = ImportRequest { source: ". \"missing/pkg\"".into(), ..request };
        assert!(import_item(&bindings, &request).is_empty());
    }

    #[test]
    fn range_assignment_and_declaration_have_different_scopes() {
        let file = parsed("a.go", "package p\nfunc f(xs []int) { for i := range xs {}\nfor i = range xs {} }");
        let syntax = Syntax::new(&file, ResolutionAdapter { language: "go".into() });
        let ranges: Vec<_> = syntax.nodes().into_iter().filter(|n| n.kind() == "range_clause").collect();
        let first = ranges[0];
        let second = ranges[1];
        let (_, declared) = binding(&syntax.adapter, &syntax, (first, syntax.scope(first)));
        let (_, assigned) = binding(&syntax.adapter, &syntax, (second, syntax.scope(second)));
        assert_eq!(declared.kind(), "for_statement");
        assert_eq!(assigned.kind(), "block");
    }

    #[test]
    fn grouped_unnamed_and_variadic_parameters_count_inputs() {
        let source = b"package p\nfunc f(a, b int, rest ...string) {}\nfunc g(int, string) {}";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("go").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let counts: Vec<_> = walk(tree.root_node())
            .filter(|n| n.kind() == "function_declaration")
            .map(|n| parameter_count(n, source))
            .collect();
        assert_eq!(counts, [Some(3), Some(2)]);
    }

    #[test]
    fn import_alias_continuations_match_python_unicode() {
        let cases = [
            ("aé \"pkg\"", Some("aé")),
            ("a² \"pkg\"", Some("a²")),
            ("$_ \"pkg\"", Some("$_")),
            ("a\u{0301} \"pkg\"", None),
            ("a\u{200c} \"pkg\"", None),
            ("é \"pkg\"", None),
        ];
        for (source, expected) in cases {
            assert_eq!(IMPORT_ALIAS.captures(source).and_then(|m| m.get(1)).map(|m| m.as_str()), expected, "{source}");
        }
    }

    #[test]
    fn unary_constructor_targets_require_address_of_literal() {
        let source = b"package p\nfunc f() { _ = &Box{}; _ = *x; _ = Box{} }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("go").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let adapter = ResolutionAdapter { language: "go".into() };
        let unary: Vec<_> = walk(tree.root_node()).filter(|n| n.kind() == "unary_expression").collect();
        let (callee, receiver) = call_target(&adapter, Some(unary[0]));
        assert_eq!(callee.unwrap().utf8_text(source).unwrap(), "Box");
        assert!(receiver.is_none());
        assert_eq!(call_target(&adapter, Some(unary[1])), (None, None));
    }
}
