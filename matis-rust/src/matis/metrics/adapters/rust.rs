//! Rust metric and resolution policies.
#[path = "rust_modules.rs"]
mod rust_modules;
use super::super::{
    receiver_syntax::type_name,
    resolution_contract::{ImportRequest, ResolutionRequest},
    resolution_syntax::Syntax,
    resolver::CallResolver,
};
use super::{
    parameters::{parameter_children, parameter_node},
    resolution::{Binding, ImportBindings},
    resolution_paths::quoted_path,
};
use crate::{
    domain::models::{Call, ParsedFile, Symbol},
    syntax::{field, named_children, text},
};
use regex::Regex;
pub use rust_modules::{RustModules, path_attributes};
use std::sync::LazyLock;
use tree_sitter::Node;

/// Count a Rust match arm unless its pattern text in `source` is exactly the wildcard `_`.
pub fn match_decision(node: Node<'_>, source: &[u8]) -> bool {
    field(node, &["pattern"]).is_none_or(|n| text(Some(n), source).trim() != "_")
}
/// Count Rust inputs excluding `self` receivers and attributes; missing/error headers yield `None`.
/// The source argument is unused.
pub fn parameter_count(node: Node<'_>, _source: &[u8]) -> Option<usize> {
    let parameters = parameter_node(node, &[]).filter(|n| !n.has_error())?;
    Some(
        parameter_children(parameters)
            .into_iter()
            .filter(|c| {
                !["self_parameter", "attribute_item"].contains(&c.kind())
                    && field(*c, &["pattern"]).unwrap_or(*c).kind() != "self"
            })
            .count(),
    )
}
/// Test whether a Rust callable's parameter list contains an explicit `self` receiver.
pub fn has_self_node(node: Node<'_>) -> bool {
    field(node, &["parameters"]).is_some_and(|p| {
        named_children(p)
            .into_iter()
            .any(|c| c.kind() == "self_parameter" || field(c, &["pattern"]).unwrap_or(c).kind() == "self")
    })
}
/// Classify an impl function as static when it lacks `self`.
/// Return `None` for missing syntax or other declaration shapes so shared detection can run.
pub fn static_symbol(syntax: &Syntax, symbol: &Symbol) -> Option<bool> {
    let node = syntax.node(symbol.start_byte, symbol.end_byte)?;
    if node.kind() == "function_item"
        && node
            .parent()
            .is_some_and(|p| p.kind() == "declaration_list" && p.parent().is_some_and(|p| p.kind() == "impl_item"))
    {
        return Some(!has_self_node(node));
    }
    None
}
/// Extract Rust struct constructors or generic-function targets, removing turbofish wrappers.
/// Return `Some((callee, receiver))` for handled shapes; `None` delegates to shared extraction.
pub fn call_target<'a>(
    adapter: &super::resolution_policy::ResolutionAdapter,
    node: Node<'a>,
) -> Option<(Option<Node<'a>>, Option<Node<'a>>)> {
    if node.kind() == "struct_expression" {
        return Some((field(node, &["name"]), None));
    }
    let mut function = field(node, &["function"])?;
    if function.kind() != "generic_function" {
        return None;
    }
    while function.kind() == "generic_function" {
        function = field(function, &["function"])?;
    }
    Some(if function.kind() == "field_expression" {
        super::generic::default_call_target(adapter, Some(function))
    } else {
        (Some(function), None)
    })
}
/// Extract a quoted path or Rust use/mod name from `source`, falling back to trimmed directive text.
pub fn import_path(source: &str) -> String {
    static PATH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:^|[^\p{L}\p{N}_])(?:use|mod)[\s\x1c-\x1f]+([\p{L}\p{N}_:]+)").unwrap());
    quoted_path(source)
        .unwrap_or_else(|| PATH.captures(source).map(|c| c[1].to_owned()).unwrap_or_else(|| source.trim().into()))
}
/// Reject explicit receiver nodes with simple Rust-like qualifiers, except `this`.
/// Missing receivers or qualifiers do not trigger this rejection.
pub fn receiver_rejected(receiver: Option<Node<'_>>, qualifier: Option<&str>) -> bool {
    static SIMPLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\A[\p{L}\p{N}_:]+\z").unwrap());
    receiver.is_some() && qualifier != Some("this") && qualifier.is_some_and(|q| SIMPLE.is_match(q))
}
/// Prefer a non-namespace import's targets when local and named candidates contain only impl symbols.
/// Otherwise retain `matches` for shared type filtering.
pub fn qualify_class(resolver: &CallResolver, request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    let local = resolver.lexical(request.file, &request.name, request.owner);
    let binding = resolver.imports.get(request.file, &request.name, request.call.start_byte);
    if let Some(binding) = binding
        && !binding.namespace
        && local.iter().chain(&matches).all(|s| s.kind == "impl")
    {
        return binding.targets;
    }
    matches
}
fn impl_types(resolver: &CallResolver, symbol: &Symbol, name: &str) -> Vec<Symbol> {
    let Some(file) = resolver.files.get(&symbol.file_path) else {
        return vec![];
    };
    let call = Call {
        file_path: symbol.file_path.clone(),
        name: name.into(),
        start_byte: symbol.start_byte,
        end_byte: symbol.end_byte,
        line: symbol.start_line,
        evidence: String::new(),
        bare_target: true,
        qualifier: None,
        relationship: "calls".into(),
    };
    let request = ResolutionRequest {
        file,
        call: &call,
        owner: resolver.symbols.get(&symbol.parent_key),
        name: name.into(),
        qualifier: None,
    };
    let mut matches = qualify_class(resolver, &request, resolver.named(&request, false));
    if let Some((qualifier, member)) = name.rsplit_once("::") {
        matches = qualified(
            resolver,
            &ResolutionRequest { name: member.into(), qualifier: Some(qualifier.into()), ..request },
        );
    }
    matches.into_iter().filter(|s| ["struct", "enum", "type"].contains(&s.kind.as_str())).collect()
}
fn inherent_impl(resolver: &CallResolver, symbol: &Symbol, parent: &Symbol) -> bool {
    if symbol.kind != "impl" {
        return false;
    }
    let Some(syntax) = resolver.syntax.get(&symbol.file_path) else {
        return false;
    };
    let Some(node) = syntax.node(symbol.start_byte, symbol.end_byte) else {
        return false;
    };
    if field(node, &["trait"]).is_some() {
        return false;
    }
    let Some(name) = type_name(field(node, &["type"]), &syntax.source) else {
        return false;
    };
    let types = impl_types(resolver, symbol, &name);
    types.len() == 1 && types[0].key == parent.key
}
/// Find direct named members and members of inherent impl blocks that uniquely resolve to `parent`.
/// Trait impls are excluded from this lookup.
pub fn instance_members(resolver: &CallResolver, parent: &Symbol, name: &str) -> Vec<Symbol> {
    let implementations: std::collections::BTreeSet<_> = resolver
        .symbols
        .values()
        .filter(|s| s.language == "rust" && inherent_impl(resolver, s, parent))
        .map(|s| s.key.as_str())
        .collect();
    let mut result = resolver.member_declarations(parent, name);
    result.extend(
        resolver
            .symbols
            .values()
            .filter(|s| implementations.contains(s.parent_key.as_str()) && s.name == name)
            .cloned(),
    );
    result
}
/// Resolve the enclosing impl type for a function with an explicit `self` parameter.
/// Missing method/impl syntax or invalid receiver context yields no types.
pub fn receiver_owner(resolver: &CallResolver, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
    let ancestors = resolver.ancestors(request.owner);
    let Some(method) = ancestors.iter().find(|s| s.kind == "function") else {
        return vec![];
    };
    let Some(owner) = ancestors.iter().find(|s| s.kind == "impl") else {
        return vec![];
    };
    let Some(method_syntax) = resolver.syntax.get(&method.file_path) else {
        return vec![];
    };
    if !method_syntax.node(method.start_byte, method.end_byte).is_some_and(has_self_node) {
        return vec![];
    }
    let Some(syntax) = resolver.syntax.get(&owner.file_path) else {
        return vec![];
    };
    let name =
        syntax.node(owner.start_byte, owner.end_byte).and_then(|n| type_name(field(n, &["type"]), &syntax.source));
    name.map(|n| impl_types(resolver, owner, &n)).unwrap_or_default()
}
/// Test whether every nonempty local candidate is a Rust module declaration in retained syntax.
pub fn local_namespace(resolver: &CallResolver, local: &[Symbol]) -> bool {
    !local.is_empty()
        && local.iter().all(|s| {
            resolver
                .syntax
                .get(&s.file_path)
                .and_then(|syntax| syntax.node(s.start_byte, s.end_byte))
                .is_some_and(|n| n.kind() == "mod_item")
        })
}
/// Find the requested member in the unique lexical Rust module named by its qualifier.
/// Missing module indexes and unresolved/ambiguous modules yield no candidates.
pub fn qualified(resolver: &CallResolver, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
    resolver
        .modules
        .rust
        .as_ref()
        .map(|index| {
            index
                .rust_symbols(request.file, request.qualifier.as_deref().unwrap_or(""), Some(request.call.start_byte))
                .into_iter()
                .filter(|s| s.name == request.name)
                .collect()
        })
        .unwrap_or_default()
}
/// Resolve a unique external module file at the request's lexical position; inline modules yield none.
pub fn import_files(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<ParsedFile> {
    ctx.modules
        .rust
        .as_ref()
        .map(|index| index.rust_import(request.file, &request.path, request.position))
        .unwrap_or_default()
}
/// Return declarations named `name` directly inside the unique requested Rust module scope.
pub fn members(ctx: &ImportBindings, request: &ImportRequest<'_>, name: &str) -> Vec<Symbol> {
    ctx.modules
        .rust
        .as_ref()
        .map(|index| {
            index
                .rust_symbols(request.file, &request.path, request.position)
                .into_iter()
                .filter(|s| s.name == name)
                .collect()
        })
        .unwrap_or_default()
}
/// Return a unique Rust module key, or a unique terminal symbol key when the path names a member.
/// Missing or ambiguous module/member targets yield `None`.
pub fn import_key(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Option<String> {
    let key = ctx.modules.rust.as_ref()?.rust_key(request.file, &request.path, request.position);
    if key.is_some() {
        return key;
    }
    let (module, name) = request.path.rsplit_once("::")?;
    let targets = members(ctx, &ImportRequest { path: module.into(), ..request.clone() }, name);
    if targets.len() == 1 { Some(targets[0].key.clone()) } else { None }
}
/// Return `(local alias, targets, namespace flag)` for a recognized Rust use path.
/// Module paths bind namespaces; terminal symbol imports bind values, retaining empty unresolved targets.
/// Unsupported directives or a missing module index yield no entries.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    static USE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\p{L}\p{N}_])use[\s\x1c-\x1f]+([\p{L}\p{N}_:]+)(?:[\s\x1c-\x1f]+as[\s\x1c-\x1f]+([\p{L}\p{N}_]+))?").unwrap()
    });
    let Some(captures) = USE.captures(&request.source) else {
        return vec![];
    };
    let path = &captures[1];
    let alias = captures.get(2).map(|m| m.as_str());
    let Some(index) = ctx.modules.rust.as_ref() else {
        return vec![];
    };
    if index.rust_key(request.file, path, request.position).is_some() {
        return vec![(
            alias.unwrap_or(path.rsplit("::").next().unwrap_or("")).into(),
            index.rust_symbols(request.file, path, request.position),
            true,
        )];
    }
    let (module, name) = path.rsplit_once("::").unwrap_or(("", path));
    vec![(
        alias.unwrap_or(name).into(),
        members(ctx, &ImportRequest { path: module.into(), ..request.clone() }, name),
        false,
    )]
}
/// Build implicit namespace bindings for Rust mod declarations in `file`.
/// Use each declaration's parent byte range, with `source_length` as fallback end; missing indexes yield none.
pub fn module_bindings(ctx: &ImportBindings, file: &ParsedFile, source_length: usize) -> Vec<Binding> {
    let Some(syntax) = ctx.syntax.get(&file.source.path) else {
        return vec![];
    };
    let Some(index) = ctx.modules.rust.as_ref() else {
        return vec![];
    };
    syntax
        .nodes()
        .into_iter()
        .filter(|n| n.kind() == "mod_item")
        .map(|node| {
            let name = text(field(node, &["name"]), &syntax.source);
            let parent = node.parent();
            Binding {
                targets: index.rust_symbols(file, &name, Some(node.start_byte())),
                name,
                namespace: true,
                start: parent.map(|p| p.start_byte()).unwrap_or(0),
                end: parent.map(|p| p.end_byte()).unwrap_or(source_length),
                members: None,
                scope: None,
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "../../../../tests/unit/metrics/adapters/rust_tests.rs"]
mod tests;
