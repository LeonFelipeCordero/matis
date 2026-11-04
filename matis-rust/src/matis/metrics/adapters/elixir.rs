//! Elixir metric boundaries, declaration identities and executable clause selection.
use super::super::{
    resolution_contract::{ImportRequest, ResolutionRequest},
    resolution_syntax::Syntax,
    resolver::CallResolver,
};
use super::{
    callable_selection::Clause,
    parameters::count_parameters,
    resolution::{CallableIdentity, ImportBindings},
    resolution_paths::quoted_path,
};
use crate::{
    domain::models::{ParsedFile, ParsedRepository, Symbol},
    language_syntax::{DECLARATIONS, declaration_head},
    syntax::{children, field, named_children, text},
};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};
use tree_sitter::Node;
pub const SCOPES: &[&str] = &[
    "def",
    "defp",
    "defmacro",
    "defmacrop",
    "defguard",
    "defguardp",
    "defdelegate",
    "defmodule",
    "defprotocol",
    "defimpl",
];

/// Return an unqualified Elixir call target identifier from matching `source`, or an empty string.
pub fn target(node: Node<'_>, source: &[u8]) -> String {
    if node.kind() != "call" {
        return String::new();
    }
    text(field(node, &["target"]).filter(|n| n.kind() == "identifier"), source)
}
/// Recognize declaration calls that create Elixir metric scope boundaries using matching `source`.
pub fn scope(node: Node<'_>, source: &[u8]) -> bool {
    SCOPES.contains(&target(node, source).as_str())
}
/// Count unqualified `if` and `unless` calls as decisions using target text in `source`.
pub fn call_decision(node: Node<'_>, source: &[u8]) -> bool {
    ["if", "unless"].contains(&target(node, source).as_str())
}
/// Identify a `when` token among the guard expression's direct children; source bytes are unused.
pub fn guard_decision(node: Node<'_>, _source: &[u8]) -> bool {
    children(node).iter().any(|c| c.kind() == "when")
}
/// Count refutable Elixir clauses, excluding anonymous-function clauses and plain fallback patterns.
/// Bare `_` and cond's `true` fallback do not count; guarded wildcards still count.
pub fn clause_decision(node: Node<'_>, source: &[u8]) -> bool {
    let Some(parent) = node.parent().filter(|n| n.kind() != "anonymous_function") else {
        return false;
    };
    let Some(pattern) = field(node, &["left"]).filter(|n| n.kind() == "arguments") else {
        return true;
    };
    let expressions = named_children(pattern);
    if expressions.len() != 1 {
        return true;
    }
    let fallback = text(Some(expressions[0]), source);
    if fallback.trim() == "_" {
        return false;
    }
    !(fallback.trim() == "true" && parent.parent().is_some_and(|owner| target(owner, source) == "cond"))
}
/// Find an arguments field or supported argument-list child, or `None` for missing/unsupported syntax.
pub fn arguments(node: Option<Node<'_>>) -> Option<Node<'_>> {
    let node = node?;
    field(node, &["arguments"]).or_else(|| {
        named_children(node).into_iter().find(|n| ["arguments", "argument_list", "value_arguments"].contains(&n.kind()))
    })
}
/// Count declaration inputs or a shared anonymous-clause arity using matching `source`.
/// Unsupported/error heads and differing anonymous-clause counts yield `None`.
pub fn parameter_count(node: Node<'_>, source: &[u8]) -> Option<usize> {
    if node.kind() == "anonymous_function" {
        return anonymous_count(node);
    }
    if node.kind() != "call" || !DECLARATIONS.contains(&target(node, source).as_str()) {
        return None;
    }
    let head = declaration_head(node).filter(|n| !n.has_error())?;
    match head.kind() {
        "identifier" => Some(0),
        "call" => count_parameters(arguments(Some(head)), &[]),
        _ => None,
    }
}
fn anonymous_count(node: Node<'_>) -> Option<usize> {
    let counts: BTreeSet<_> = named_children(node)
        .into_iter()
        .filter(|n| n.kind() == "stab_clause")
        .map(|n| {
            let Some(mut inputs) = field(n, &["left"]) else {
                return Some(0);
            };
            if inputs.kind() == "binary_operator" {
                inputs = field(inputs, &["left"])?;
            }
            if inputs.kind() != "arguments" {
                return None;
            }
            count_parameters(Some(inputs), &[])
        })
        .collect();
    if counts.len() == 1 { counts.into_iter().next().flatten() } else { None }
}
/// Count named argument nodes, treating calls without arguments as zero and unsupported forms as `None`.
pub fn arity(node: Option<Node<'_>>) -> Option<usize> {
    arguments(node).map(|a| named_children(a).len()).or_else(|| node.filter(|n| n.kind() == "call").map(|_| 0))
}
/// Return call arity including one implicit input when the node is a pipeline's right operand.
/// Read pipeline spelling from matching `source`; unsupported nodes yield `None`.
pub fn call_arity(node: Option<Node<'_>>, source: &[u8]) -> Option<usize> {
    let pipeline = node.is_some_and(|n| {
        n.parent().is_some_and(|p| {
            p.kind() == "binary_operator" && field(p, &["right"]) == Some(n) && text(Some(p), source).contains("|>")
        })
    });
    let count = arity(node).or_else(|| node.filter(|n| n.kind() == "identifier" && pipeline).map(|_| 0))?;
    Some(count + usize::from(pipeline))
}
fn clause_signature(node: Option<Node<'_>>, source: &[u8]) -> Clause {
    let Some(node) = node else {
        return Clause { minimum: 0, maximum: 0, private: false };
    };
    let mut head = arguments(Some(node)).and_then(|a| named_children(a).first().copied());
    if head.is_some_and(|n| n.kind() == "binary_operator") {
        head = head.and_then(|n| field(n, &["left"]));
    }
    let parameters = arguments(head).map(named_children).unwrap_or_default();
    let maximum = parameters.len();
    let defaults = parameters.iter().filter(|n| text(Some(**n), source).contains("\\\\")).count();
    Clause { minimum: maximum - defaults, maximum, private: text(field(node, &["target"]), source) == "defp" }
}
/// Extract a quoted path or Elixir import/alias/require/use name from `source`.
/// Fall back to trimmed directive text when no pattern matches.
pub fn import_path(source: &str) -> String {
    static PATH: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?:^|[^\p{L}\p{N}_])(?:import|alias|require|use)[\s\x1c-\x1f]+([\p{L}\p{N}_.:/]+)").unwrap()
    });
    quoted_path(source)
        .unwrap_or_else(|| PATH.captures(source).map(|c| c[1].into()).unwrap_or_else(|| source.trim().into()))
}
/// Recognize Elixir `alias` and `import` calls using their target spelling in `source`.
pub fn is_import_node(node: Node<'_>, source: &[u8]) -> bool {
    node.kind() == "call" && ["alias", "import"].contains(&text(field(node, &["target"]), source).as_str())
}
/// Return module names directly, otherwise the final `::` component of a declaration's qualified name.
/// The file and syntax arguments are unused.
pub fn module_identity(_file: &ParsedFile, symbol: &Symbol, _syntax: &Syntax) -> String {
    if symbol.kind == "module" {
        symbol.name.clone()
    } else {
        symbol.qualified_name.rsplit("::").next().unwrap_or("").into()
    }
}
/// Return the uniquely declared Elixir import target key, or `None` for missing/ambiguous targets.
pub fn import_key(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Option<String> {
    let targets = ctx.modules.declared(request.file, &request.path);
    if targets.len() == 1 { Some(targets[0].key.clone()) } else { None }
}
/// Return one `(alias or terminal name, declaration targets, false)` Elixir binding.
/// Preserve unresolved and ambiguous module-index targets.
pub fn import_item(ctx: &ImportBindings, request: &ImportRequest<'_>) -> Vec<(String, Vec<Symbol>, bool)> {
    static ALIAS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:^|[^\p{L}\p{N}_])as[\s\x1c-\x1f]*:?[\s\x1c-\x1f]*([\p{L}\p{N}_]+)").unwrap());
    let alias = ALIAS
        .captures(&request.source)
        .map(|c| c[1].to_owned())
        .unwrap_or_else(|| request.path.rsplit('.').next().unwrap_or("").into());
    vec![(alias, ctx.modules.declared(request.file, &request.path), false)]
}
/// Resolve the requested member as a static member of the module named by its qualifier.
pub fn qualified(resolver: &CallResolver, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
    resolver.members(
        &resolver.modules.declared(request.file, request.qualifier.as_deref().unwrap_or("")),
        &request.name,
        true,
    )
}
/// Index Elixir function/method minimum/maximum arity and privacy by symbol key.
/// Read declaration heads from `syntax`; missing syntax uses zero arity and public visibility.
pub fn clauses(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>) -> BTreeMap<String, Clause> {
    repository
        .symbols()
        .filter(|s| s.language == "elixir" && ["function", "method"].contains(&s.kind.as_str()))
        .map(|symbol| {
            let clause = syntax
                .get(&symbol.file_path)
                .map(|syntax| clause_signature(syntax.node(symbol.start_byte, symbol.end_byte), &syntax.source))
                .unwrap_or(Clause { minimum: 0, maximum: 0, private: false });
            (symbol.key.clone(), clause)
        })
        .collect()
}
/// Select Elixir clauses by pipeline-aware call arity and private-call visibility.
/// Retain all viable clauses in one parent/name group; ambiguity between groups yields none.
/// Unknown arity accepts only an already unique candidate.
pub fn select(identity: &CallableIdentity, request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    let syntax = &identity.syntax[&request.file.source.path];
    let node = syntax.node(request.call.start_byte, request.call.end_byte);
    let Some(count) = call_arity(node, &syntax.source) else {
        return if matches.len() == 1 { matches } else { vec![] };
    };
    let matches: Vec<_> = matches
        .into_iter()
        .filter(|s| {
            identity.clauses.get(&s.key).is_some_and(|c| {
                c.minimum <= count
                    && count <= c.maximum
                    && (!c.private
                        || (request.call.qualifier.as_deref().unwrap_or("").is_empty()
                            && s.file_path == request.call.file_path))
            })
        })
        .collect();
    let identities: BTreeSet<_> = matches.iter().map(|s| (&s.parent_key, &s.name)).collect();
    if identities.len() == 1 { matches } else { vec![] }
}

#[derive(Clone)]
pub struct ElixirModules {
    pub repository: ParsedRepository,
    pub syntax: BTreeMap<String, Syntax>,
}
impl ElixirModules {
    /// Retain repository facts and shared syntax for Elixir module-identity lookup.
    pub fn new(repository: &ParsedRepository, syntax: &BTreeMap<String, Syntax>) -> Self {
        Self { repository: repository.clone(), syntax: syntax.clone() }
    }
    /// Return declarations with exact module identity `name` and `source`'s language, preserving ambiguity.
    pub fn declared(&self, source: &ParsedFile, name: &str) -> Vec<Symbol> {
        self.repository
            .files
            .iter()
            .filter(|f| f.source.language == source.source.language)
            .flat_map(|f| {
                f.symbols.iter().filter(|s| module_identity(f, s, &self.syntax[&f.source.path]) == name).cloned()
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/metrics/adapters/elixir_tests.rs"]
mod tests;
