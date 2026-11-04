//! Python metrics, lexical scope, module imports and callable identity policies.
use super::parameters::{count_parameters, parameter_children, parameter_node};
use super::resolution::{CallableIdentity, ImportBindings, ResolutionAdapter};
use super::resolution_paths::{path_target_key, quoted_path, resolve_path, top_symbols};
use super::script_python_selection::{bound_self, import_names, overload_implementation, typing_overload};
use super::script_resolution::{inherited_members, same_declaration_group};
use crate::domain::models::{ParsedFile, ParsedRepository, Symbol};
use crate::metrics::resolution_contract::{DeclarationLookup, ImportRequest, ResolutionRequest};
use crate::metrics::resolution_syntax::Syntax;
use crate::metrics::resolver::CallResolver;
use crate::syntax::{children, field, named_children, text};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tree_sitter::Node;

static IMPORT_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^\p{L}\p{N}_])from[\s\x1c-\x1f]+([\p{L}\p{N}_.]+)|(?:^|[^\p{L}\p{N}_])import[\s\x1c-\x1f]+([\p{L}\p{N}_.]+)").unwrap()
});

pub const COMPREHENSIONS: &[&str] =
    &["list_comprehension", "set_comprehension", "dictionary_comprehension", "generator_expression"];

/// Recognize wildcard/capture patterns and supported wrappers that cannot reject a Python match.
/// Unsupported pattern shapes are treated as refutable.
pub fn irrefutable_pattern(pattern: Node<'_>) -> bool {
    let named = named_children(pattern);
    let single = || single_pattern(pattern);
    let wildcard = || children(pattern).iter().any(|c| c.kind() == "_");
    match pattern.kind() {
        "case_pattern" => case_pattern(pattern),
        "dotted_name" => capture_pattern(pattern),
        "as_pattern" => named.first().is_some_and(|n| irrefutable_pattern(*n)),
        "union_pattern" => wildcard() || named.into_iter().any(irrefutable_pattern),
        "tuple_pattern" => !children(pattern).iter().any(|n| n.kind() == ",") && single(),
        _ => false,
    }
}
fn case_pattern(pattern: Node<'_>) -> bool {
    if pattern.named_child_count() == 0 {
        children(pattern).iter().any(|c| c.kind() == "_")
    } else {
        single_pattern(pattern)
    }
}
fn single_pattern(pattern: Node<'_>) -> bool {
    pattern.named_child_count() == 1 && pattern.named_child(0).is_some_and(irrefutable_pattern)
}
fn capture_pattern(pattern: Node<'_>) -> bool {
    pattern.named_child_count() == 1 && pattern.named_child(0).is_some_and(|n| n.kind() == "identifier")
}
/// Count a Python case as a decision unless its case pattern is recognized as irrefutable.
/// A missing case pattern still counts as a decision.
pub fn case_decision(node: Node<'_>) -> bool {
    named_children(node).into_iter().find(|c| c.kind() == "case_pattern").is_none_or(|p| !irrefutable_pattern(p))
}
/// Count explicit Python inputs, excluding separators and the first receiver of non-static methods.
/// Use matching `source` for decorators; missing/error headers yield `None`, except bare lambdas yield zero.
pub fn parameter_count(node: Node<'_>, source: &[u8]) -> Option<usize> {
    let Some(parameters) = parameter_node(node, &[]) else {
        return (node.kind() == "lambda").then_some(0);
    };
    let count = count_parameters(Some(parameters), &["positional_separator", "keyword_separator"])?;
    let entries = parameter_children(parameters);
    Some(count - usize::from(entries.first().is_some_and(|n| python_method(node, source) && python_receiver(*n))))
}
fn python_method(node: Node<'_>, source: &[u8]) -> bool {
    let mut owner = node.parent();
    if let Some(decorated) = owner.filter(|n| n.kind() == "decorated_definition") {
        if named_children(decorated)
            .into_iter()
            .filter(|c| c.kind() == "decorator")
            .any(|c| text(parameter_children(c).first().copied(), source) == "staticmethod")
        {
            return false;
        }
        owner = decorated.parent();
    }
    owner.is_some_and(|n| n.kind() == "block" && n.parent().is_some_and(|p| p.kind() == "class_definition"))
}
fn python_receiver(mut node: Node<'_>) -> bool {
    if node.kind() == "typed_parameter" {
        node = node.named_child(0).unwrap_or(node);
    }
    ["identifier", "default_parameter", "typed_default_parameter"].contains(&node.kind())
}

/// Extract the last quoted path or Python from/import module name from `source`.
/// Fall back to trimmed directive text when no pattern matches.
pub fn import_path(source: &str) -> String {
    quoted_path(source).unwrap_or_else(|| {
        IMPORT_PATH
            .captures(source)
            .and_then(|c| c.get(1).or_else(|| c.get(2)))
            .map(|m| m.as_str().into())
            .unwrap_or_else(|| source.trim().into())
    })
}
/// Find named Python instance members through a supported single-base chain.
/// Reject decorated classes, unsafe mutations, masking fields, and ambiguous inheritance.
pub fn instance_members(resolver: &CallResolver, parent: &Symbol, name: &str) -> Vec<Symbol> {
    inherited_members(resolver, parent, name)
}
/// Select Python runtime callables, rejecting unbound `self` calls and typing-only overloads.
/// Multiple candidates must share a declaration group with one final non-overload implementation.
pub fn select(identity: &CallableIdentity, request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    if request.qualifier.as_deref().is_some_and(|q| q.split('.').next() == Some("self")) {
        let syntax = &identity.syntax[&request.file.source.path];
        if !bound_self(syntax, syntax.node(request.call.start_byte, request.call.end_byte)) {
            return vec![];
        }
    }
    if matches.len() <= 1 {
        return matches.into_iter().filter(|s| !typing_overload(&identity.syntax[&s.file_path], s)).collect();
    }
    if !same_declaration_group(&matches) {
        return vec![];
    }
    let mut ordered = matches;
    ordered.sort_by_key(|s| s.start_byte);
    overload_implementation(&identity.syntax[&ordered[0].file_path], ordered)
}

fn parent(path: &Path) -> PathBuf {
    path.parent().unwrap_or(Path::new("")).to_owned()
}
fn source_roots(files: &[&ParsedFile]) -> BTreeSet<PathBuf> {
    let packages: BTreeSet<_> = files
        .iter()
        .filter(|f| Path::new(&f.source.path).file_name().is_some_and(|n| n == "__init__.py"))
        .map(|f| parent(Path::new(&f.source.path)))
        .collect();
    let mut roots = BTreeSet::from([PathBuf::new()]);
    roots.extend(packages.iter().map(|package| package_root(package.clone(), &packages)));
    for file in files {
        roots.extend(
            Path::new(&file.source.path)
                .ancestors()
                .skip(1)
                .filter(|p| p.file_name().is_some_and(|n| n == "src"))
                .map(Path::to_owned),
        );
    }
    roots
}
fn package_root(mut package: PathBuf, packages: &BTreeSet<PathBuf>) -> PathBuf {
    while !package.as_os_str().is_empty() && packages.contains(&package) {
        package = parent(&package);
    }
    package
}
/// Map Python module identities to retained `.py` files, preserving ambiguous identities.
/// Derive roots from package initializers, `src` directories, and the repository root;
/// the syntax argument is unused.
pub fn module_index(
    repository: &ParsedRepository,
    _syntax: &BTreeMap<String, Syntax>,
) -> BTreeMap<String, Vec<ParsedFile>> {
    let files: Vec<_> =
        repository.files.iter().filter(|f| f.source.language == "python" && f.source.path.ends_with(".py")).collect();
    let roots = source_roots(&files);
    let mut result: BTreeMap<String, Vec<ParsedFile>> = BTreeMap::new();
    for (file, name) in files.into_iter().flat_map(|file| {
        roots.iter().filter_map(move |root| module_name(Path::new(&file.source.path), root).map(|name| (file, name)))
    }) {
        result.entry(name).or_default().push(file.clone());
    }
    result
}
fn module_name(path: &Path, root: &Path) -> Option<String> {
    let mut module = path.strip_prefix(root).ok()?.with_extension("");
    if module.file_name().is_some_and(|n| n == "__init__") {
        module = parent(&module);
    }
    let name = module.iter().map(|s| s.to_string_lossy()).collect::<Vec<_>>().join(".");
    (!name.is_empty()).then_some(name)
}
fn package_parent(files: &[ParsedFile]) -> bool {
    files.is_empty()
        || files.len() == 1 && Path::new(&files[0].source.path).file_name().is_some_and(|n| n == "__init__.py")
}
/// Resolve a unique Python module through `ctx`'s module index or relative-path lookup.
/// Reject ambiguous modules and absolute paths whose indexed parents are not packages.
pub fn import_files(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<ParsedFile> {
    if request.path.starts_with('.') {
        return ctx.path_files(request.file, &request.path);
    }
    let index = &ctx.modules.python;
    let parts: Vec<_> = request.path.split('.').collect();
    if (1..parts.len())
        .any(|length| !package_parent(index.get(&parts[..length].join(".")).map(Vec::as_slice).unwrap_or(&[])))
    {
        return vec![];
    }
    let matches = index.get(&request.path).cloned().unwrap_or_default();
    if matches.len() == 1 { matches } else { vec![] }
}
/// Match Python module names to `.py` or `__init__.py` files, relative to `source` for leading dots.
/// Return all matching retained files; path normalization may query filesystem metadata.
pub fn paths(source: &ParsedFile, name: &str, files: &[ParsedFile]) -> Vec<ParsedFile> {
    let dots = name.len() - name.trim_start_matches('.').len();
    let module = name[dots..].replace('.', "/");
    if dots == 0 {
        let targets = [format!("{module}.py"), format!("{module}/__init__.py")];
        return files.iter().filter(|f| targets.contains(&f.source.path)).cloned().collect();
    }
    let mut base = parent(&source.source.absolute_path);
    for _ in 1..dots {
        base = parent(&base);
    }
    let base = resolve_path(&base.join(module));
    let targets = [base.with_extension("py"), base.join("__init__.py")];
    files.iter().filter(|f| targets.contains(&resolve_path(&f.source.absolute_path))).cloned().collect()
}

/// Return `(local import name, enclosing scope)` pairs from Python import syntax for shadowing checks.
pub fn lexical_imports(syntax: &Syntax) -> Vec<(String, Node<'_>)> {
    let adapter = &syntax.adapter;
    syntax
        .nodes()
        .into_iter()
        .filter(|n| adapter.import_nodes().contains(&n.kind()))
        .flat_map(|node| {
            let mut cursor = node.walk();
            node.children_by_field_name("name", &mut cursor)
                .map(|child| {
                    let name = field(child, &["alias"])
                        .map(|a| syntax.text(a))
                        .unwrap_or_else(|| syntax.text(child).split('.').next().unwrap_or("").into());
                    (name, syntax.scope(node))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
/// Extract Python declaration, exception, assignment, or loop targets with their binding scope.
/// Move named-expression bindings outside comprehension scopes; unsupported nodes have no target.
pub fn binding<'a>(
    adapter: &ResolutionAdapter,
    syntax: &'a Syntax,
    (node, mut scope): (Node<'a>, Node<'a>),
) -> (Option<Node<'a>>, Node<'a>) {
    if ["function_definition", "class_definition"].contains(&node.kind()) {
        return (field(node, &["name"]), scope);
    }
    if ["except_clause", "as_pattern"].contains(&node.kind()) {
        return (field(node, &["alias", "parameter"]), scope);
    }
    let target = if adapter.assignments().contains(&node.kind()) {
        field(node, &["left", "name"]).or_else(|| node.named_child(0))
    } else if adapter.loop_bindings().contains(&node.kind()) {
        field(node, &["left", "name"])
    } else {
        None
    };
    if node.kind() == "named_expression" {
        while COMPREHENSIONS.contains(&scope.kind()) {
            scope = syntax.scope(scope);
        }
    }
    (target, scope)
}
/// Apply Python scope restrictions at byte `position` after range containment is checked by the caller.
/// Hide comprehension captures in the first iterable and class locals inside nested executable bodies.
pub fn visible(syntax: &Syntax, scope: Node<'_>, position: usize) -> bool {
    if COMPREHENSIONS.contains(&scope.kind()) {
        let iterable =
            named_children(scope).into_iter().find(|c| c.kind() == "for_in_clause").and_then(|c| field(c, &["right"]));
        if iterable.is_some_and(|n| n.start_byte() <= position && position < n.end_byte()) {
            return false;
        }
    }
    if scope.kind() != "class_definition" || syntax.root().is_none() {
        return true;
    }
    if field(scope, &["body"]).is_some_and(|n| !(n.start_byte() <= position && position < n.end_byte())) {
        return false;
    }
    let mut parent = syntax.root().and_then(|n| n.descendant_for_byte_range(position, position));
    while let Some(node) = parent.filter(|n| *n != scope) {
        if hides_class_scope(syntax, node, position) {
            return false;
        }
        parent = node.parent();
    }
    true
}
fn hides_class_scope(syntax: &Syntax, node: Node<'_>, position: usize) -> bool {
    if ["function_definition", "lambda", "class_definition"].contains(&node.kind())
        && field(node, &["body"]).is_some_and(|n| n.start_byte() <= position && position < n.end_byte())
    {
        return true;
    }
    COMPREHENSIONS.contains(&node.kind()) && syntax.visible(node, position)
}
/// Exempt a class declaration from shadowing unless a same-name import is visible at the lookup position.
pub fn declaration_allowed(syntax: &Syntax, lookup: &DeclarationLookup<'_>) -> bool {
    lookup.node.kind() == "class_definition"
        && !syntax
            .imports()
            .into_iter()
            .any(|(alias, scope)| alias == lookup.name && syntax.visible(scope, lookup.position))
}
/// Detect a class-scope import named `name` that is hidden from a nested body at byte `position`.
pub fn import_shadowed(syntax: &Syntax, name: &str, position: usize) -> bool {
    syntax.imports().into_iter().any(|(alias, scope)| {
        alias == name
            && scope.kind() == "class_definition"
            && field(scope, &["body"]).is_some_and(|n| n.start_byte() <= position && position < n.end_byte())
            && !syntax.visible(scope, position)
    })
}
/// Treat exact `@staticmethod`/`@classmethod` decorators as static, otherwise use shared modifier detection.
pub fn static_symbol(syntax: &Syntax, symbol: &Symbol) -> bool {
    let adapter = &syntax.adapter;
    let node = syntax.node(symbol.start_byte, symbol.end_byte);
    if let Some(parent) = node.and_then(|n| n.parent()).filter(|n| n.kind() == "decorated_definition") {
        return named_children(parent)
            .into_iter()
            .filter(|c| c.kind() == "decorator")
            .any(|c| ["@staticmethod", "@classmethod"].contains(&syntax.text(c).trim()));
    }
    adapter.default_static_symbol(syntax, symbol)
}

fn import_node<'a>(ctx: &'a ImportBindings, request: &ImportRequest<'_>) -> Option<Node<'a>> {
    ctx.syntax[&request.file.source.path].nodes().into_iter().find(|n| {
        Some(n.start_byte()) == request.position && ["import_statement", "import_from_statement"].contains(&n.kind())
    })
}
fn child_path(path: &str, name: &str) -> String {
    format!("{path}{}{name}", if path.ends_with('.') { "" } else { "." })
}
fn module_attribute(ctx: &ImportBindings, request: &ImportRequest<'_>, name: &str) -> bool {
    let modules = ctx.files(request.file, &request.path, None);
    if modules.len() != 1 {
        return false;
    }
    let syntax = &ctx.syntax[&modules[0].source.path];
    let assigned = syntax.bindings_named(name).into_iter().any(|(scope, node)| {
        Some(scope) == syntax.root() && (node.kind() != "assignment" || field(node, &["right"]).is_some())
    });
    let excluded = if modules[0].source.path == request.file.source.path { request.position } else { None };
    assigned
        || syntax.nodes().into_iter().any(|n| {
            syntax.adapter.import_nodes().contains(&n.kind())
                && Some(n.start_byte()) != excluded
                && Some(syntax.scope(n)) == syntax.root()
                && import_names(n, &syntax.source).into_iter().any(|(imported, alias)| {
                    (if alias.is_empty() { imported.split('.').next().unwrap_or("") } else { &alias }) == name
                })
        })
}
fn child_module_files(ctx: &ImportBindings, request: &ImportRequest<'_>, name: &str) -> Vec<ParsedFile> {
    if !package_parent(&ctx.files(request.file, &request.path, None)) {
        return vec![];
    }
    let unpositioned = ImportRequest { position: None, ..request.clone() };
    if !ctx.members(&unpositioned, name).is_empty() || module_attribute(ctx, request, name) {
        return vec![];
    }
    ctx.files(request.file, &child_path(&request.path, name), None)
}
fn module_symbols(ctx: &ImportBindings, file: &ParsedFile, path: &str) -> Vec<Symbol> {
    ctx.files(file, path, None).iter().flat_map(top_symbols).collect()
}
/// Return `(module path, optional graph key)` dependencies for Python import directives.
/// Expand imported child modules when appropriate and retain unresolved dependencies with `None` keys.
pub fn import_targets(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Option<String>)> {
    let adapter = &ctx.syntax[&request.file.source.path].adapter;
    let Some(node) = import_node(ctx, request) else {
        return vec![(request.path.clone(), adapter.import_key(ctx, request))];
    };
    let names = import_names(node, &ctx.syntax[&request.file.source.path].source);
    if node.kind() == "import_statement" {
        return names
            .into_iter()
            .map(|(name, _)| {
                let key = path_target_key(&ctx.files(request.file, &name, None));
                (name, key)
            })
            .collect();
    }
    let mut result = vec![];
    for (name, _) in names {
        let target = from_import_target(ctx, request, &name);
        if !result.contains(&target) {
            result.push(target);
        }
    }
    if result.is_empty() { vec![(request.path.clone(), adapter.import_key(ctx, request))] } else { result }
}
fn from_import_target(ctx: &ImportBindings, request: &ImportRequest<'_>, name: &str) -> (String, Option<String>) {
    let adapter = &ctx.syntax[&request.file.source.path].adapter;
    let members = ctx.members(&ImportRequest { position: None, ..request.clone() }, name);
    let child = child_module_files(ctx, request, name);
    if !request.path.is_empty()
        && request.path.chars().all(|c| c == '.')
        && request.source.trim_start().starts_with("from ")
    {
        return (
            child_path(&request.path, name),
            if members.len() == 1 { Some(members[0].key.clone()) } else { path_target_key(&child) },
        );
    }
    if !child.is_empty() {
        return (child_path(&request.path, name), path_target_key(&child));
    }
    (request.path.clone(), adapter.import_key(ctx, request))
}
/// Return `(local name, targets, namespace flag)` entries for Python imports at the request position.
/// Prefer from-import members over child modules; unresolved recognized names retain empty targets.
/// A missing directive node yields no bindings.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    let Some(node) = import_node(ctx, request) else {
        return vec![];
    };
    import_names(node, &ctx.syntax[&request.file.source.path].source)
        .into_iter()
        .map(|(name, alias)| {
            let local = if alias.is_empty() { name.clone() } else { alias };
            if node.kind() != "import_from_statement" {
                return (local, module_symbols(ctx, request.file, &name), true);
            }
            let members = ctx.members(&ImportRequest { position: None, ..request.clone() }, &name);
            if !members.is_empty() {
                return (local, members, false);
            }
            if child_module_files(ctx, request, &name).is_empty() {
                return (local, vec![], false);
            }
            (local, module_symbols(ctx, request.file, &child_path(&request.path, &name)), true)
        })
        .collect()
}
/// Permit `self` only when it is the method's first parameter and has no conflicting visible binding.
/// Requests with other qualifiers are permitted without extra checks.
pub fn implicit_receiver(resolver: &CallResolver, request: &ResolutionRequest<'_>, method: &Symbol) -> bool {
    if request.qualifier.as_deref() != Some("self") {
        return true;
    }
    let syntax = &resolver.syntax[&request.file.source.path];
    let parameters = syntax.node(method.start_byte, method.end_byte).and_then(|n| field(n, &["parameters"]));
    if parameters.and_then(|n| n.named_child(0)).is_none_or(|n| syntax.text(n) != "self") {
        return false;
    }
    !syntax
        .bindings_at("self", request.call.start_byte)
        .into_iter()
        .any(|(_, declaration)| Some(declaration) != parameters)
}

#[cfg(test)]
#[path = "../../../../tests/unit/metrics/adapters/script_adapter_tests.rs"]
mod script_adapter_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn match_patterns_and_method_inputs() {
        let source = b"class C:\n def f(self, x, /, *, y): pass\n @staticmethod\n def g(x): pass\nmatch x:\n case _: pass\n case 1: pass\n";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("python").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let nodes: Vec<_> = crate::syntax::walk(tree.root_node()).collect();
        let functions: Vec<_> = nodes.iter().filter(|n| n.kind() == "function_definition").collect();
        assert_eq!(parameter_count(*functions[0], source), Some(2));
        assert_eq!(parameter_count(*functions[1], source), Some(1));
        let cases: Vec<_> = nodes.into_iter().filter(|n| n.kind() == "case_clause").map(case_decision).collect();
        assert_eq!(cases, vec![false, true]);
    }
    #[test]
    fn import_word_classes_match_python_unicode_rules() {
        assert_eq!(import_path("import é²"), "é²");
        assert_eq!(import_path("import e\u{301}"), "e");
        assert_eq!(import_path("x\u{301}import value"), "value");
        assert_eq!(import_path("import\u{1c}value"), "value");
    }
    #[test]
    fn roots_keep_package_and_source_identities() {
        assert_eq!(module_name(Path::new("src/pkg/__init__.py"), Path::new("src")), Some("pkg".into()));
        assert_eq!(module_name(Path::new("src/pkg/a.py"), Path::new("")), Some("src.pkg.a".into()));
        assert_eq!(module_name(Path::new("__init__.py"), Path::new("")), None);
    }
}
