//! JavaScript and JSX runtime exports, imports, methods and receiver policies.
use super::parameters::{parameter_children, parameter_node};
use super::resolution::{CallableIdentity, ImportBindings, ResolutionAdapter};
use super::resolution_paths::{quoted_path, relative_targets, top_symbols};
use super::script_resolution::{inherited_members, member_mutation, member_write, same_declaration_group};
use crate::domain::models::{ParsedFile, Symbol};
use crate::metrics::resolution_contract::{FUNCTIONS, ImportRequest, ResolutionRequest};
use crate::metrics::resolution_syntax::Syntax;
use crate::metrics::resolver::CallResolver;
use crate::syntax::{children, field, named_children, text};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;
use tree_sitter::Node;

static EXPORT_TYPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export[\s\x1c-\x1f]+type(?:$|[^\p{L}\p{N}_])").unwrap());
static EXPORT_DEFAULT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^export[\s\x1c-\x1f]+default(?:$|[^\p{L}\p{N}_])").unwrap());
static IMPORT_CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)(?:^|[^\p{L}\p{N}_])import[\s\x1c-\x1f]+(.+?)[\s\x1c-\x1f]+from(?:$|[^\p{L}\p{N}_])").unwrap()
});
static NAMED_CLAUSE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{([^}]+)\}").unwrap());
static NAMED_IMPORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[\s\x1c-\x1f]*(type[\s\x1c-\x1f]+)?([\p{L}\p{N}_$]+)(?:[\s\x1c-\x1f]+as[\s\x1c-\x1f]+([\p{L}\p{N}_$]+))?[\s\x1c-\x1f]*$").unwrap()
});
static NAMESPACE_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\*[\s\x1c-\x1f]+as[\s\x1c-\x1f]+([\p{L}\p{N}_$]+)").unwrap());
static DEFAULT_IMPORT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\p{L}\p{N}_$]+)(?:[\s\x1c-\x1f]*,|$)").unwrap());

/// Count explicit inputs except TypeScript `this`; missing/error headers yield `None`.
/// A single identifier parameter counts as one; the source argument is unused.
pub fn parameter_count(node: Node<'_>, _source: &[u8]) -> Option<usize> {
    let parameters = parameter_node(node, &[])?;
    if parameters.has_error() {
        return None;
    }
    if parameters.kind() == "identifier" {
        return Some(1);
    }
    Some(
        parameter_children(parameters)
            .into_iter()
            .filter(|c| c.kind() != "this" && field(*c, &["pattern"]).unwrap_or(*c).kind() != "this")
            .count(),
    )
}
fn local_export(node: Node<'_>, source: &[u8]) -> bool {
    node.kind() == "export_statement"
        && field(node, &["source"]).is_none()
        && !EXPORT_TYPE.is_match(&text(Some(node), source))
}
fn declaration_symbols(node: Node<'_>, declarations: &[Symbol], source: &[u8]) -> Vec<Symbol> {
    let name = text(Some(node), source);
    declarations
        .iter()
        .filter(|s| s.name == name || node.start_byte() <= s.start_byte && s.start_byte < node.end_byte())
        .cloned()
        .collect()
}
fn export_entries(node: Node<'_>, declarations: &[Symbol], source: &[u8]) -> Vec<(String, Vec<Symbol>)> {
    let mut entries = vec![];
    if let Some(declaration) = field(node, &["declaration", "value"]) {
        let default = EXPORT_DEFAULT.is_match(&text(Some(node), source));
        entries.extend(
            declaration_symbols(declaration, declarations, source)
                .into_iter()
                .map(|s| (if default { "default".into() } else { s.name.clone() }, vec![s])),
        );
    }
    for spec in named_children(node).into_iter().filter(|c| c.kind() == "export_clause").flat_map(named_children) {
        if text(Some(spec), source).starts_with("type ") {
            continue;
        }
        let original = text(field(spec, &["name"]), source);
        let alias = text(field(spec, &["alias"]), source);
        entries.push((
            if alias.is_empty() { original.clone() } else { alias },
            declarations.iter().filter(|s| s.name == original).cloned().collect(),
        ));
    }
    entries
}
/// Map local runtime export names in `syntax` to unique top-level symbols from `file`.
/// Exclude re-exports and type-only exports; retain aliases and default exports.
pub fn export_bindings(file: &ParsedFile, syntax: &Syntax) -> BTreeMap<String, Vec<Symbol>> {
    ordered_export_bindings(file, syntax).into_iter().collect()
}
fn ordered_export_bindings(file: &ParsedFile, syntax: &Syntax) -> Vec<(String, Vec<Symbol>)> {
    let declarations = top_symbols(file);
    let mut result: Vec<(String, Vec<Symbol>)> = vec![];
    for (name, symbols) in syntax
        .nodes()
        .into_iter()
        .filter(|n| local_export(*n, &syntax.source))
        .flat_map(|n| export_entries(n, &declarations, &syntax.source))
    {
        if let Some((_, targets)) = result.iter_mut().find(|(alias, _)| *alias == name) {
            targets.extend(symbols);
        } else {
            result.push((name, symbols));
        }
    }
    for (_, symbols) in &mut result {
        *symbols = unique_symbols(std::mem::take(symbols));
    }
    result
}
fn unique_symbols(symbols: Vec<Symbol>) -> Vec<Symbol> {
    let mut unique: Vec<Symbol> = vec![];
    for symbol in symbols {
        if let Some(existing) = unique.iter_mut().find(|s| s.key == symbol.key) {
            *existing = symbol;
        } else {
            unique.push(symbol);
        }
    }
    unique
}
fn import_clause(source: &str) -> String {
    IMPORT_CLAUSE.captures(source).map(|c| c[1].trim().to_owned()).unwrap_or_default()
}
fn named_imports(ctx: &ImportBindings, request: &ImportRequest<'_>, raw: &str) -> Vec<(String, Vec<Symbol>, bool)> {
    let Some(named) = NAMED_CLAUSE.captures(raw) else {
        return vec![];
    };
    named[1]
        .split(',')
        .filter_map(|part| {
            let found = NAMED_IMPORT.captures(part)?;
            let targets = if import_clause(&request.source).starts_with("type ") || found.get(1).is_some() {
                vec![]
            } else {
                ctx.members(request, &found[2])
            };
            Some((found.get(3).map(|m| m.as_str()).unwrap_or(&found[2]).to_owned(), targets, false))
        })
        .collect()
}
fn has_type_modifier(node: Node<'_>) -> bool {
    children(node).iter().any(|c| c.kind() == "type")
}
/// Test whether an import has a whole-statement type modifier or only type-marked named specifiers.
/// Missing/non-import nodes and mixed runtime imports return false.
pub fn import_type_only(node: Option<Node<'_>>) -> bool {
    let Some(node) = node.filter(|n| n.kind() == "import_statement") else {
        return false;
    };
    if has_type_modifier(node) {
        return true;
    }
    let clause = named_children(node).into_iter().find(|n| n.kind() == "import_clause");
    let named = clause
        .filter(|n| n.named_child_count() == 1)
        .and_then(|n| n.named_child(0))
        .filter(|n| n.kind() == "named_imports");
    let specifiers: Vec<_> =
        named.map(named_children).unwrap_or_default().into_iter().filter(|n| n.kind() == "import_specifier").collect();
    !specifiers.is_empty() && specifiers.into_iter().all(has_type_modifier)
}
fn field_masks_method(child: Node<'_>, (name, implementations): (&str, &BTreeSet<String>), source: &[u8]) -> bool {
    let Some(target) = field(child, &["name", "property"]) else {
        return false;
    };
    if target.kind() == "computed_property_name" {
        return true;
    }
    text(Some(target), source) == name
        && (["field_definition", "public_field_definition"].contains(&child.kind())
            || ["method_signature", "abstract_method_signature"].contains(&child.kind())
                && !implementations.contains(name)
            || children(child).iter().any(|c| ["get", "set"].contains(&c.kind())))
}
fn own_field(syntax: &Syntax, parent: &Symbol, name: &str) -> bool {
    let Some(body) = syntax.node(parent.start_byte, parent.end_byte).and_then(|n| field(n, &["body"])) else {
        return false;
    };
    let children = named_children(body);
    let implementations = children
        .iter()
        .filter(|c| field(**c, &["body"]).is_some())
        .map(|c| text(field(*c, &["name"]), &syntax.source))
        .collect();
    children.into_iter().any(|child| field_masks_method(child, (name, &implementations), &syntax.source))
}
fn receiver_alias(node: Node<'_>, receivers: &BTreeSet<String>, source: &[u8]) -> Option<String> {
    if !["variable_declarator", "assignment_expression"].contains(&node.kind())
        || !receivers.contains(&text(field(node, &["value", "right"]), source))
    {
        return None;
    }
    field(node, &["name", "left"]).filter(|n| n.kind() == "identifier").map(|n| text(Some(n), source))
}
fn script_mutation(syntax: &Syntax, parent: &Symbol, name: &str) -> bool {
    let nodes = syntax.nodes();
    let mut receivers = BTreeSet::from([parent.name.clone(), format!("{}.prototype", parent.name)]);
    for _ in &nodes {
        let aliases =
            nodes.iter().filter_map(|n| receiver_alias(*n, &receivers, &syntax.source)).collect::<BTreeSet<_>>();
        if aliases.is_subset(&receivers) {
            break;
        }
        receivers.extend(aliases);
    }
    nodes.into_iter().any(|node| {
        syntax.adapter.member_writes().contains(&node.kind())
            && member_write(&text(syntax.adapter.write_target(node), &syntax.source), &receivers, name)
            || member_mutation(node, &receivers, &syntax.source)
    })
}
/// Find instance members along a supported single-base chain, rejecting mutations and masking fields.
pub fn instance_members(resolver: &CallResolver, parent: &Symbol, name: &str) -> Vec<Symbol> {
    inherited_members(resolver, parent, name)
}
/// Retain a unique candidate or the final plain method in one same-staticness declaration group.
/// Accessors, mixed scopes, and other ambiguities yield no targets; the request argument is unused.
pub fn select(identity: &CallableIdentity, _request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    if matches.len() <= 1 {
        return matches;
    }
    if !same_declaration_group(&matches) || !matches.iter().all(|s| s.kind == "method") {
        return vec![];
    }
    let plain = matches.iter().all(|s| {
        identity.syntax[&s.file_path].node(s.start_byte, s.end_byte).is_some_and(|n| {
            n.kind() == "method_definition"
                && field(n, &["body"]).is_some()
                && !children(n).iter().any(|c| ["get", "set"].contains(&c.kind()))
        })
    });
    let statuses: BTreeSet<_> = matches.iter().map(|s| identity.syntax[&s.file_path].static_symbol(s)).collect();
    if !plain || statuses.len() != 1 {
        return vec![];
    }
    let last = matches.iter().map(|s| s.start_byte).max().unwrap();
    matches.into_iter().find(|s| s.start_byte == last).into_iter().collect()
}
/// Return the last quoted import path in `source`, falling back to trimmed directive text.
pub fn import_path(source: &str) -> String {
    quoted_path(source).unwrap_or_else(|| source.trim().to_owned())
}
/// Return `(callee, receiver)` for calls; `new` expressions use the constructor/type with no receiver.
/// Other forms use shared target extraction with `adapter` metadata.
pub fn call_target<'a>(adapter: &ResolutionAdapter, node: Option<Node<'a>>) -> (Option<Node<'a>>, Option<Node<'a>>) {
    if let Some(node) = node.filter(|n| n.kind() == "new_expression") {
        return (field(node, &["constructor", "type"]), None);
    }
    super::generic::default_call_target(adapter, node)
}
/// Permit class-expression receiver inference only if its prototype member is not rebound in `syntax`.
pub fn receiver_class_allowed(syntax: &Syntax, callee: Option<Node<'_>>, member: &str) -> bool {
    !syntax.member_rebound(&format!("{}.prototype", text(callee, &syntax.source)), member)
}
/// Detect rebinding, alias/prototype mutation, or fields/accessors that mask `name` on `parent`.
pub fn member_overwritten(syntax: &Syntax, parent: &Symbol, name: &str) -> bool {
    syntax.adapter.default_member_overwritten(syntax, parent, name)
        || script_mutation(syntax, parent, name)
        || own_field(syntax, parent, name)
}
/// Extract script binding targets and scopes, keeping named function-expression names inside the function.
/// Hoist `var` targets to the nearest function/root scope; unsupported nodes have no target.
pub fn binding<'a>(
    adapter: &ResolutionAdapter,
    syntax: &'a Syntax,
    (node, mut scope): (Node<'a>, Node<'a>),
) -> (Option<Node<'a>>, Node<'a>) {
    if node.kind() == "function_declaration" {
        return (field(node, &["name"]), scope);
    }
    if node.kind() == "function_expression" {
        return (field(node, &["name"]), node);
    }
    let (target, _) = super::generic::binding(adapter, syntax, (node, scope));
    if node.kind() == "variable_declarator" && node.parent().is_some_and(|p| syntax.text(p).starts_with("var ")) {
        while scope.parent().is_some() && !FUNCTIONS.contains(&scope.kind()) {
            scope = syntax.scope(scope);
        }
    }
    (target, scope)
}
/// Resolve dot-prefixed slash imports relative to `source`, allowing directory `index` files only.
/// Bare package names yield no targets; normalization may query filesystem metadata.
pub fn paths(source: &ParsedFile, name: &str, files: &[ParsedFile]) -> Vec<ParsedFile> {
    if name.starts_with('.') && name.contains('/') {
        relative_targets(
            &ImportRequest { file: source, path: name.into(), position: None, source: String::new() },
            files,
            true,
        )
    } else {
        vec![]
    }
}
/// Return runtime exports named `name` only when the requested import resolves to one file.
pub fn members(ctx: &ImportBindings, request: &ImportRequest<'_>, name: &str) -> Vec<Symbol> {
    let targets = ctx.path_files(request.file, &request.path);
    if targets.len() != 1 {
        return vec![];
    }
    export_bindings(&targets[0], &ctx.syntax[&targets[0].source.path]).remove(name).unwrap_or_default()
}
/// Merge runtime export aliases from files resolved by `path` relative to `file` in `ctx`.
/// Preserve multiple symbol candidates under the same export name.
pub fn namespace_exports(ctx: &ImportBindings, file: &ParsedFile, path: &str) -> BTreeMap<String, Vec<Symbol>> {
    ordered_namespace_exports(ctx, file, path).into_iter().collect()
}
fn ordered_namespace_exports(ctx: &ImportBindings, file: &ParsedFile, path: &str) -> Vec<(String, Vec<Symbol>)> {
    let mut result: Vec<(String, Vec<Symbol>)> = vec![];
    for (name, symbols) in ctx
        .files(file, path, None)
        .into_iter()
        .flat_map(|target| ordered_export_bindings(&target, &ctx.syntax[&target.source.path]))
    {
        if let Some((_, targets)) = result.iter_mut().find(|(alias, _)| *alias == name) {
            targets.extend(symbols);
        } else {
            result.push((name, symbols));
        }
    }
    result
}
/// Return `(local name, symbols, namespace flag)` entries for namespace, named, and default imports.
/// Type-only imports retain bindings with empty targets; side-effect/unsupported imports yield none.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    let clause = import_clause(&request.source);
    if clause.is_empty() {
        return vec![];
    }
    let type_only = clause.starts_with("type ");
    let raw = clause.strip_prefix("type ").unwrap_or(&clause);
    let mut result = vec![];
    if let Some(namespace) = NAMESPACE_IMPORT.captures(raw) {
        let targets = ordered_namespace_exports(ctx, request.file, &request.path)
            .into_iter()
            .flat_map(|(_, symbols)| symbols)
            .collect();
        result.push((namespace[1].into(), if type_only { vec![] } else { targets }, true));
    }
    result.extend(named_imports(ctx, request, raw));
    if let Some(default) = DEFAULT_IMPORT.captures(raw) {
        result.push((default[1].into(), if type_only { vec![] } else { ctx.members(request, "default") }, false));
    }
    result
}
/// Return namespace export aliases for the request, or an empty map when `targets` is empty.
pub fn namespace_members(
    ctx: &ImportBindings,
    request: &ImportRequest<'_>,
    targets: &[Symbol],
) -> BTreeMap<String, Vec<Symbol>> {
    if targets.is_empty() { BTreeMap::new() } else { namespace_exports(ctx, request.file, &request.path) }
}
/// Permit `this` only when scopes nested inside `method` are arrow functions that retain its receiver.
/// Other qualifiers require no extra check; missing method ancestry rejects `this`.
pub fn implicit_receiver(resolver: &CallResolver, request: &ResolutionRequest<'_>, method: &Symbol) -> bool {
    if request.qualifier.as_deref() != Some("this") {
        return true;
    }
    let ancestors = resolver.ancestors(request.owner);
    let Some(position) = ancestors.iter().position(|s| s.key == method.key) else {
        return false;
    };
    ancestors[..position].iter().all(|s| {
        resolver.syntax[&request.file.source.path]
            .node(s.start_byte, s.end_byte)
            .is_some_and(|n| n.kind() == "arrow_function")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_inputs_and_type_imports() {
        let source = b"import {type Foo, type Bar} from './a';\nconst f = (this: Foo, x: number) => x;";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("typescript").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let nodes: Vec<_> = crate::syntax::walk(tree.root_node()).collect();
        assert!(import_type_only(nodes.iter().copied().find(|n| n.kind() == "import_statement")));
        let arrow = nodes.into_iter().find(|n| n.kind() == "arrow_function").unwrap();
        assert_eq!(parameter_count(arrow, source), Some(1));
    }
    #[test]
    fn regex_word_classes_match_python_unicode_rules() {
        assert!(NAMED_IMPORT.is_match("é² as value"));
        assert!(!NAMED_IMPORT.is_match("e\u{301} as value"));
        assert_eq!(import_clause("x\u{301}import value from './a'"), "value");
        assert!(EXPORT_TYPE.is_match("export type\u{301}"));
        assert!(NAMED_IMPORT.is_match("\u{1c}value\u{1f}"));
    }
    #[test]
    fn import_paths_and_clauses() {
        assert_eq!(import_path("import A from './a'"), "./a");
        assert_eq!(import_clause("import type {A as B} from './a'"), "type {A as B}");
        assert_eq!(import_clause("import './a'"), "");
    }
}
