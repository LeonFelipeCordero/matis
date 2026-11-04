//! Native Java metric and resolution hooks.
use super::generic;
use super::parameters::{count_parameters, parameter_node};
use super::resolution::{CallableIdentity, ImportBindings, Modules};
use super::resolution_paths::quoted_path;
use super::resolution_policy::ResolutionAdapter;
use crate::domain::models::{ParsedFile, Symbol};
use crate::metrics::resolution_contract::{ImportRequest, ResolutionRequest};
use crate::metrics::resolution_syntax::Syntax;
use crate::metrics::resolver::CallResolver;
use crate::syntax::{field, named_children};
use regex::Regex;
use std::sync::LazyLock;
use tree_sitter::Node;

static IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\A|[^\p{L}\p{N}_])import\s+(?:static\s+)?([\p{L}\p{N}_.]+)").unwrap());
static PACKAGE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\A|[^\p{L}\p{N}_])package\s+([\p{L}\p{N}_.]+)").unwrap());

/// Count Java inputs excluding receiver parameters; a single lambda identifier counts as one.
/// Missing/error containers yield `None`; the source argument is unused.
pub fn parameter_count(node: Node<'_>, _source: &[u8]) -> Option<usize> {
    let parameters = parameter_node(node, &[]);
    if parameters.is_some_and(|n| n.kind() == "identifier") {
        return Some(1);
    }
    count_parameters(parameters, &["receiver_parameter"])
}

/// Extract the package name from Java/Kotlin syntax, or an empty string when absent.
pub fn package(syntax: &Syntax) -> String {
    let source = syntax
        .nodes()
        .into_iter()
        .find(|n| ["package_declaration", "package_header"].contains(&n.kind()))
        .map(|n| syntax.text(n))
        .unwrap_or_default();
    PACKAGE.captures(&source).map(|m| m[1].to_owned()).unwrap_or_default()
}

/// Extract the last quoted path or Java import name, including static imports.
/// Fall back to trimmed `source` when no pattern matches.
pub fn import_path(source: &str) -> String {
    quoted_path(source)
        .unwrap_or_else(|| IMPORT.captures(source).map(|m| m[1].to_owned()).unwrap_or_else(|| source.trim().into()))
}

/// Return the sole viable JVM callable after conservative argument-shape and proven-type filtering.
/// Unknown signatures remain viable; unresolved ambiguity yields no targets.
pub fn select(identity: &CallableIdentity, request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    super::callable_selection::select_callable(identity, request, matches)
}

/// Return `(callee, receiver)` for Java calls, constructors, and two-part method references.
/// Constructors have no receiver; other shapes use shared extraction with `adapter` metadata.
pub fn call_target<'a>(adapter: &ResolutionAdapter, node: Option<Node<'a>>) -> (Option<Node<'a>>, Option<Node<'a>>) {
    if let Some(node) = node {
        if node.kind() == "object_creation_expression" {
            return (field(node, &["constructor", "type"]), None);
        }
        let children = named_children(node);
        if node.kind() == "method_reference" && children.len() == 2 {
            return (children.last().copied(), children.first().copied());
        }
    }
    generic::default_call_target(adapter, node)
}

/// Extract Java binding targets, giving enhanced-for declarations the loop node as scope.
/// Catch parameters retain the input scope; other forms use shared extraction.
pub fn binding<'a>(
    adapter: &ResolutionAdapter,
    syntax: &'a Syntax,
    (node, scope): (Node<'a>, Node<'a>),
) -> (Option<Node<'a>>, Node<'a>) {
    match node.kind() {
        "enhanced_for_statement" => (field(node, &["name"]), node),
        "catch_formal_parameter" => (field(node, &["name"]), scope),
        _ => generic::binding(adapter, syntax, (node, scope)),
    }
}

/// Return `symbol`'s package-qualified identity using Java/Kotlin `syntax`; the file argument is unused.
pub fn module_identity(_file: &ParsedFile, symbol: &Symbol, syntax: &Syntax) -> String {
    identity_in_package(symbol, &package(syntax))
}
/// Join `package` with the final `::` component of the symbol's qualified name, omitting empty parts.
pub fn identity_in_package(symbol: &Symbol, package: &str) -> String {
    let name = symbol.qualified_name.rsplit("::").next().unwrap_or("");
    [package, name].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(".")
}

/// Find declarations named `name` in `source`'s indexed package and language, preserving ambiguity.
pub fn same_package(modules: &Modules, source: &ParsedFile, name: &str) -> Vec<Symbol> {
    let Some(package) = modules.packages.get(&source.source.path) else {
        return vec![];
    };
    let identity = [package.as_str(), name].into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join(".");
    modules.declared(source, &identity)
}

/// Extract a Java import's qualified name from `source`, using `path` as fallback.
pub fn import_name(source: &str, path: &str) -> String {
    IMPORT.captures(source).map(|m| m[1].to_owned()).unwrap_or_else(|| path.into())
}

/// Return the unique imported declaration key, or `None` for missing or ambiguous targets.
/// Kotlin uses the request path directly; Java parses the directive's qualified name.
pub fn import_key(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Option<String> {
    let path = if request.file.source.language == "kotlin" {
        request.path.clone()
    } else {
        import_name(&request.source, &request.path)
    };
    let targets = ctx.modules.declared(request.file, &path);
    (targets.len() == 1).then(|| targets[0].key.clone())
}

/// Return one `(terminal import name, declaration targets, false)` binding for the request.
/// Preserve unresolved and ambiguous declaration targets.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    let path = import_name(&request.source, &request.path);
    vec![(path.rsplit('.').next().unwrap_or("").into(), ctx.modules.declared(request.file, &path), false)]
}

/// Resolve dotted request names through the module declaration index; otherwise retain `matches`.
pub fn qualify_class(resolver: &CallResolver, request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    if request.name.contains('.') { resolver.modules.declared(request.file, &request.name) } else { matches }
}

/// Resolve dotted inherited names through package declarations and `::` names through class lookup.
/// Retain `matches` for unqualified names.
pub fn inheritance(resolver: &CallResolver, request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    if request.name.contains('.') {
        return resolver.modules.declared(request.file, &request.name);
    }
    if request.name.contains("::") {
        return resolver.class_named(request);
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::walk;

    fn parsed(path: &str, language: &str, source: &str) -> ParsedFile {
        crate::parsing::parser::parse_file(crate::domain::models::SourceFile {
            path: path.into(),
            absolute_path: std::path::PathBuf::from(path),
            language: language.into(),
            source: source.as_bytes().to_vec(),
        })
        .unwrap()
    }

    #[test]
    fn packages_and_imports_preserve_ambiguous_targets() {
        let file = parsed("Main.java", "java", "package demo; class Main {}");
        let target = parsed("Box.java", "java", "package demo; class Box {}");
        let duplicate = parsed("Other.java", "java", "package demo; class Box {}");
        let foreign = parsed("Other.kt", "kotlin", "package demo\nclass Box {}");
        let repository =
            crate::domain::models::ParsedRepository { files: vec![file.clone(), target, duplicate, foreign] };
        let syntax = repository
            .files
            .iter()
            .map(|f| (f.source.path.clone(), Syntax::new(f, ResolutionAdapter { language: f.source.language.clone() })))
            .collect();
        let modules = Modules::new(&repository, &syntax);
        assert_eq!(package(&syntax["Main.java"]), "demo");
        assert_eq!(same_package(&modules, &file, "Box").len(), 2);
        let bindings = ImportBindings::new(&repository, &syntax, &modules);
        let request =
            ImportRequest { file: &file, path: "demo.Box".into(), source: "import demo.Box;".into(), position: None };
        assert!(import_key(&bindings, &request).is_none());
        let entries = import_item(&bindings, &request);
        assert_eq!(entries[0].0, "Box");
        assert_eq!(entries[0].1.len(), 2);
        assert!(!entries[0].2);
    }

    #[test]
    fn imports_keep_static_and_quoted_path_behavior() {
        assert_eq!(import_path("import static a.B.run;"), "a.B.run");
        assert_eq!(import_path("import a.B; // 'preferred'"), "preferred");
        assert_eq!(import_path(" not an import "), "not an import");
    }

    #[test]
    fn directive_words_and_boundaries_match_python_unicode() {
        let cases = [
            ("\u{0301}import café.Type;", "café.Type"),
            ("\u{200c}import pkg.Type;", "pkg.Type"),
            ("éimport pkg.Type;", "fallback"),
            ("²import pkg.Type;", "fallback"),
            ("_import pkg.Type;", "fallback"),
            ("import cafe\u{0301}.Type;", "cafe"),
            ("import a\u{200c}.Type;", "a"),
            ("import a².Type;", "a².Type"),
            ("ximport import Good;", "Good"),
        ];
        for (source, expected) in cases {
            assert_eq!(import_name(source, "fallback"), expected, "{source}");
        }
        assert_eq!(PACKAGE.captures("\u{0301}package café.Type;").unwrap().get(1).unwrap().as_str(), "café.Type");
        assert!(PACKAGE.captures("épackage ignored;").is_none());
    }

    #[test]
    fn receiver_parameters_and_single_lambda_inputs() {
        let source = b"class A { void f(A this, int x, String... rest) {} void g() { F f = x -> x; } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("java").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let method = walk(tree.root_node()).find(|n| n.kind() == "method_declaration").unwrap();
        assert_eq!(parameter_count(method, source), Some(2));
        let lambda = walk(tree.root_node()).find(|n| n.kind() == "lambda_expression").unwrap();
        assert_eq!(parameter_count(lambda, source), Some(1));
    }

    #[test]
    fn constructors_and_references_have_native_targets() {
        let source = b"class A { void f() { new Box(); F f = Box::run; } }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("java").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let adapter = ResolutionAdapter { language: "java".into() };
        let creation = walk(tree.root_node()).find(|n| n.kind() == "object_creation_expression").unwrap();
        let (callee, receiver) = call_target(&adapter, Some(creation));
        assert_eq!(callee.unwrap().utf8_text(source).unwrap(), "Box");
        assert!(receiver.is_none());
        let reference = walk(tree.root_node()).find(|n| n.kind() == "method_reference").unwrap();
        let (callee, receiver) = call_target(&adapter, Some(reference));
        assert_eq!(callee.unwrap().utf8_text(source).unwrap(), "run");
        assert_eq!(receiver.unwrap().utf8_text(source).unwrap(), "Box");
    }
}
