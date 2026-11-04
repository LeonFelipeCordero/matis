//! Conservative repository-local import, inheritance, and callable resolution.
use super::adapters::{
    get_resolution_adapter,
    resolution::{Binding, CallableIdentity, ImportBindings, Modules},
};
use super::receiver_resolution::ReceiverTypes;
use super::resolution_contract::{CallTarget, ResolutionRequest};
use super::resolution_syntax::{IDENTIFIERS, Syntax};
use crate::domain::models::{Call, ParsedFile, ParsedRepository, Symbol, file_key};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::LazyLock;

static QUALIFIED_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_$]+(?:\.|::)[\p{L}\p{N}_.$:]+$").unwrap());
static SCOPED_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\p{L}\p{N}_.:]+)(?:\.|::)([\p{L}\p{N}_$!?]+)$").unwrap());
use tree_sitter::Node;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalNode {
    pub key: String,
    pub language: String,
    pub kind: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Connection {
    pub source_key: String,
    pub target_key: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub confidence: String,
    pub path: String,
    pub line: usize,
    pub evidence: Option<String>,
    pub type_only: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ResolvedGraph {
    pub connections: Vec<Connection>,
    pub external_nodes: Vec<ExternalNode>,
}
pub const TYPES: &[&str] = &["class", "struct", "enum", "object"];
pub const CALLABLES: &[&str] = &["function", "method", "constructor", "class", "struct", "enum", "object"];

#[path = "resolver_graph.rs"]
mod graph;
pub use graph::resolve_connections;

fn import_masks_local(file: &ParsedFile, binding: Option<&Binding>, local: &[Symbol]) -> bool {
    binding.is_some_and(|b| b.start == 0) && local.iter().any(|s| s.parent_key == file_key(&file.source.path))
}
fn is_bare_target(call: &Call, callee: Option<Node<'_>>) -> bool {
    call.bare_target || callee.is_some_and(|n| IDENTIFIERS.contains(&n.kind()) || n.kind() == "type_identifier")
}

pub struct CallResolver {
    pub repository: ParsedRepository,
    pub files: BTreeMap<String, ParsedFile>,
    pub symbols: BTreeMap<String, Symbol>,
    pub syntax: BTreeMap<String, Syntax>,
    pub modules: Modules,
    pub imports: ImportBindings,
    pub identity: CallableIdentity,
    pub receivers: ReceiverTypes,
}
pub type Resolver = CallResolver;
impl CallResolver {
    /// Clone repository facts and build module, import, callable, and lazy receiver indexes.
    /// Syntax parsing uses retained source rather than filesystem reads.
    pub fn new(repository: &ParsedRepository) -> Self {
        let files = repository.files.iter().map(|f| (f.source.path.clone(), f.clone())).collect();
        let symbols = repository.symbols().map(|s| (s.key.clone(), s.clone())).collect();
        let syntax = repository
            .files
            .iter()
            .map(|f| (f.source.path.clone(), Syntax::new(f, get_resolution_adapter(&f.source.language))))
            .collect();
        let modules = Modules::new(repository, &syntax);
        let imports = ImportBindings::new(repository, &syntax, &modules);
        let identity = CallableIdentity::new(repository, &syntax);
        let receivers = ReceiverTypes::new(&syntax);
        Self { repository: repository.clone(), files, symbols, syntax, modules, imports, identity, receivers }
    }
    /// Return the narrowest symbol containing the call's full byte span, or `None`.
    /// The call's file must belong to this repository.
    pub fn owner(&self, call: &Call) -> Option<&Symbol> {
        self.files[&call.file_path]
            .symbols
            .iter()
            .filter(|s| s.start_byte <= call.start_byte && call.end_byte <= s.end_byte)
            .min_by_key(|s| s.end_byte - s.start_byte)
    }
    /// Return `owner` and its symbol ancestors, nearest first, stopping at missing parents or cycles.
    pub fn ancestors<'a>(&'a self, mut owner: Option<&'a Symbol>) -> Vec<Symbol> {
        let mut result: Vec<Symbol> = Vec::new();
        while let Some(symbol) = owner {
            if result.iter().any(|s| s.key == symbol.key) {
                break;
            }
            result.push(symbol.clone());
            owner = self.symbols.get(&symbol.parent_key);
        }
        result
    }
    /// Find `name` in the nearest nonempty enclosing declaration scope, then the file scope.
    /// Apply language rules that exclude type scopes from lexical lookup.
    pub fn lexical(&self, file: &ParsedFile, name: &str, owner: Option<&Symbol>) -> Vec<Symbol> {
        let scopes = self.ancestors(owner).into_iter().map(|s| s.key).chain([file_key(&file.source.path)]);
        for scope in scopes {
            let matches = self.scope_declarations(file, name, &scope);
            if !matches.is_empty() {
                return matches;
            }
        }
        Vec::new()
    }
    fn scope_declarations(&self, file: &ParsedFile, name: &str, scope: &str) -> Vec<Symbol> {
        if self.symbols.get(scope).is_some_and(|parent| TYPES.contains(&parent.kind.as_str()))
            && get_resolution_adapter(&file.source.language).skip_type_scope()
        {
            return Vec::new();
        }
        file.symbols.iter().filter(|s| s.name == name && s.parent_key == scope).cloned().collect()
    }
    /// Resolve an unqualified request through lexical declarations, imports, and package peers.
    /// Reject shadowed names; `fallback` permits same-language repository callable lookup last.
    pub fn named(&self, request: &ResolutionRequest<'_>, fallback: bool) -> Vec<Symbol> {
        let file = request.file;
        let binding = self.imports.get(file, &request.name, request.call.start_byte);
        let local = self.lexical(file, &request.name, request.owner);
        if self.syntax[&file.source.path].shadowed(&request.name, request.call.start_byte, &local) {
            return Vec::new();
        }
        if !local.is_empty() {
            return if import_masks_local(file, binding.as_ref(), &local) { Vec::new() } else { local };
        }
        if let Some(binding) = binding {
            return if binding.namespace { Vec::new() } else { binding.targets };
        }
        let package = self.modules.same_package(file, &request.name);
        if !package.is_empty() || !fallback {
            return package;
        }
        self.fallback_named(file, &request.name)
    }
    fn fallback_named(&self, file: &ParsedFile, name: &str) -> Vec<Symbol> {
        let adapter = get_resolution_adapter(&file.source.language);
        let family = if adapter.family().is_empty() { &file.source.language } else { adapter.family() };
        self.repository
            .symbols()
            .filter(|s| s.name == name)
            .filter(|s| {
                let candidate = get_resolution_adapter(&s.language);
                let language = if candidate.family().is_empty() { &s.language } else { candidate.family() };
                language == family && CALLABLES.contains(&s.kind.as_str()) && !TYPES.contains(&s.kind.as_str())
            })
            .cloned()
            .collect()
    }
    /// Resolve type candidates without global fallback and apply language qualification rules.
    pub fn class_named(&self, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let matches = self.named(request, false);
        get_resolution_adapter(&request.file.source.language)
            .qualify_class(self, request, matches)
            .into_iter()
            .filter(|s| TYPES.contains(&s.kind.as_str()))
            .collect()
    }
    fn receiver_class(&self, request: &ResolutionRequest<'_>, receiver: Option<Node<'_>>) -> Vec<Symbol> {
        let Some(receiver) = receiver else {
            return Vec::new();
        };
        let adapter = get_resolution_adapter(&request.file.source.language);
        if !adapter.receiver_class_nodes().contains(receiver.kind()) {
            return Vec::new();
        }
        let (callee, nested) = adapter.call_target(Some(receiver));
        if nested.is_some() {
            return Vec::new();
        }
        let syntax = &self.syntax[&request.file.source.path];
        if !adapter.receiver_class_allowed(syntax, callee, &request.name) {
            return Vec::new();
        }
        let mut request = request.clone();
        request.name = callee.map(|n| syntax.text(n)).unwrap_or_default();
        self.class_named(&request)
    }
    /// Return the enclosing type for a permitted implicit instance receiver, or no candidates.
    /// Static methods and invalid implicit receivers are excluded.
    pub fn self_class(&self, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let ancestors = self.ancestors(request.owner);
        let Some(parent) = ancestors.iter().find(|s| TYPES.contains(&s.kind.as_str())) else {
            return Vec::new();
        };
        let Some(method) = ancestors.iter().find(|s| s.parent_key == parent.key) else {
            return Vec::new();
        };
        if self.syntax[&request.file.source.path].static_symbol(method) {
            return Vec::new();
        }
        if !get_resolution_adapter(&request.file.source.language).implicit_receiver(self, request, method) {
            return Vec::new();
        }
        vec![parent.clone()]
    }
    /// Find `name` on exactly one unmodified type; ambiguity or rebinding yields no candidates.
    /// When `static_` is true, retain only static or namespace-eligible members.
    pub fn members(&self, types: &[Symbol], name: &str, static_: bool) -> Vec<Symbol> {
        if types.len() != 1 {
            return Vec::new();
        }
        let parent = &types[0];
        let syntax = &self.syntax[&parent.file_path];
        let adapter = get_resolution_adapter(&parent.language);
        if adapter.member_overwritten(syntax, parent, name) {
            return Vec::new();
        }
        let declarations = adapter.instance_members(self, parent, name);
        if !static_ {
            return declarations;
        }
        declarations
            .into_iter()
            .filter(|s| self.syntax[&s.file_path].static_symbol(s) || adapter.static_member(parent))
            .collect()
    }
    /// Return direct callable declarations named `name` under `parent`, without inheritance lookup.
    pub fn member_declarations(&self, parent: &Symbol, name: &str) -> Vec<Symbol> {
        self.files[&parent.file_path]
            .symbols
            .iter()
            .filter(|s| s.parent_key == parent.key && s.name == name && CALLABLES.contains(&s.kind.as_str()))
            .cloned()
            .collect()
    }
    fn namespace_member(&self, request: &ResolutionRequest<'_>, binding: &Binding) -> Vec<Symbol> {
        let qualifier = request.qualifier.as_deref().unwrap_or("");
        let local = self.lexical(request.file, qualifier, request.owner);
        if !local.is_empty() && !get_resolution_adapter(&request.file.source.language).local_namespace(self, &local) {
            let mut class = request.clone();
            class.name = qualifier.into();
            return self.members(&self.class_named(&class), &request.name, true);
        }
        binding.member(&request.name)
    }
    /// Resolve a qualified member through self, import namespaces, or named types.
    /// Reject shadowed qualifiers and overwritten members rather than guessing.
    pub fn qualified(&self, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let qualifier = request.qualifier.as_deref().unwrap_or("");
        let syntax = &self.syntax[&request.file.source.path];
        if syntax.member_rebound(qualifier, &request.name) {
            return Vec::new();
        }
        if syntax.adapter.self_qualifiers().contains(qualifier) {
            return self.members(&self.self_class(request), &request.name, false);
        }
        if syntax.shadowed(qualifier.split('.').next().unwrap_or(""), request.call.start_byte, &[]) {
            return Vec::new();
        }
        self.qualified_binding(request)
    }
    fn qualified_binding(&self, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let qualifier = request.qualifier.as_deref().unwrap_or("");
        let binding = self.imports.get(request.file, qualifier, request.call.start_byte);
        if let Some(binding) = binding.as_ref().filter(|b| b.namespace) {
            return self.namespace_member(request, binding);
        }
        let mut class = request.clone();
        class.name = qualifier.into();
        let types = self.class_named(&class);
        if !types.is_empty() {
            return self.members(&types, &request.name, true);
        }
        if let Some(binding) = binding {
            return self.members(&binding.targets, &request.name, true);
        }
        get_resolution_adapter(&request.file.source.language).qualified(self, request)
    }
    fn matches(&self, request: &ResolutionRequest<'_>, node: Option<Node<'_>>, target: &CallTarget<'_>) -> Vec<Symbol> {
        let syntax = &self.syntax[&request.file.source.path];
        let adapter = &syntax.adapter;
        if let Some(typed) = self.typed_matches(request) {
            return typed;
        }
        if adapter.receiver_rejected(target.receiver, request.qualifier.as_deref()) {
            return Vec::new();
        }
        if let Some(receiver) = target.receiver
            && !IDENTIFIERS.contains(&receiver.kind())
            && !adapter.receiver_names().contains(receiver.kind())
        {
            return self.receiver_matches(request, receiver);
        }
        if request.qualifier.as_deref().is_some_and(|q| !q.is_empty()) {
            return self.qualified(request);
        }
        if is_bare_target(request.call, target.callee) {
            let constructor = node.is_some_and(|n| adapter.constructors().contains(n.kind()));
            return self.named(request, !constructor);
        }
        Vec::new()
    }
    fn typed_matches(&self, request: &ResolutionRequest<'_>) -> Option<Vec<Symbol>> {
        let qualifier = request.qualifier.as_deref().filter(|q| !q.is_empty())?;
        if self.syntax[&request.file.source.path].member_rebound(qualifier, &request.name) {
            return None;
        }
        ReceiverTypes::resolve(self, request)
    }
    fn receiver_matches(&self, request: &ResolutionRequest<'_>, receiver: Node<'_>) -> Vec<Symbol> {
        if QUALIFIED_PATTERN.is_match(request.qualifier.as_deref().unwrap_or("")) {
            return self.qualified(request);
        }
        self.members(&self.receiver_class(request, Some(receiver)), &request.name, false)
    }
    /// Resolve a call in optional lexical `owner` and apply language callable-selection rules.
    /// Return accepted local symbols, or an empty vector for unresolved, unsafe, or ambiguous calls.
    /// The call's file must belong to this repository; macros are not resolved.
    pub fn resolve(&self, call: &Call, owner: Option<&Symbol>) -> Vec<Symbol> {
        let file = &self.files[&call.file_path];
        let syntax = &self.syntax[&call.file_path];
        let node = syntax.node(call.start_byte, call.end_byte);
        let adapter = &syntax.adapter;
        if node.is_some_and(|n| adapter.macro_nodes().contains(n.kind())) {
            return Vec::new();
        }
        let (callee, receiver) = adapter.call_target(node);
        let target = adapter.normalized_call(syntax, call, (callee, receiver));
        let mut request =
            ResolutionRequest { file, call, owner, name: target.name.clone(), qualifier: target.qualifier.clone() };
        if target.receiver.is_none()
            && let Some(scoped) = SCOPED_PATTERN.captures(&target.name)
        {
            request.qualifier = Some(scoped[1].into());
            request.name = scoped[2].into();
        }
        let matches = self
            .matches(&request, node, &target)
            .into_iter()
            .filter(|s| CALLABLES.contains(&s.kind.as_str()) || s.kind == "module")
            .collect();
        self.identity.select(file, call, matches)
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/metrics/resolver_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_repository_is_empty_graph() {
        let graph = resolve_connections(&ParsedRepository::default()).unwrap();
        assert!(graph.connections.is_empty());
        assert!(graph.external_nodes.is_empty());
    }
}
