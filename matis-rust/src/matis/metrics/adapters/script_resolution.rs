//! Conservative inheritance, receiver mutation, and executable selection for scripts.
use super::resolution::CallableIdentity;
use crate::domain::models::{Call, Symbol};
use crate::metrics::resolution_contract::ResolutionRequest;
use crate::metrics::resolution_syntax::{ReceiverScopeData, Syntax};
use crate::metrics::resolver::CallResolver;
use crate::syntax::{children, field, named_children, text};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;
use tree_sitter::Node;

static PLAIN_BASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[\p{L}\p{N}_$]+(?:\.[\p{L}\p{N}_$]+)*$").unwrap());

/// Find named instance members through a supported plain single-base class chain.
/// Reject cycles, ambiguous bases, decorated classes, masking declarations, and receiver/member mutations.
pub fn inherited_members(resolver: &CallResolver, parent: &Symbol, name: &str) -> Vec<Symbol> {
    InheritedMember { resolver, name }.members(parent, BTreeSet::new())
}

struct InheritedMember<'a> {
    resolver: &'a CallResolver,
    name: &'a str,
}
impl InheritedMember<'_> {
    fn members(&self, parent: &Symbol, mut seen: BTreeSet<String>) -> Vec<Symbol> {
        let (resolver, name) = (self.resolver, self.name);
        if !seen.insert(parent.key.clone()) {
            return vec![];
        }
        let syntax = &resolver.syntax[&parent.file_path];
        if !plain_class(syntax, parent)
            || syntax.adapter.member_overwritten(syntax, parent, name)
            || instance_member_write(syntax, parent, name)
        {
            return vec![];
        }
        let direct = resolver.member_declarations(parent, name);
        if !direct.is_empty() {
            return if self.base_instance_write(parent, seen) { vec![] } else { direct };
        }
        if syntax.file.symbols.iter().any(|s| s.parent_key == parent.key && s.name == name) {
            return vec![];
        }
        base_class(resolver, parent).map(|base| self.members(&base, seen)).unwrap_or_default()
    }

    fn base_instance_write(&self, parent: &Symbol, mut seen: BTreeSet<String>) -> bool {
        let (resolver, name) = (self.resolver, self.name);
        let Some(target) = base_class(resolver, parent) else {
            return false;
        };
        if !seen.insert(target.key.clone()) {
            return false;
        }
        let syntax = &resolver.syntax[&target.file_path];
        !plain_class(syntax, &target)
            || instance_field(syntax, &target, name)
            || instance_member_write(syntax, &target, name)
            || self.base_instance_write(&target, seen)
    }
}

fn base_class(resolver: &CallResolver, parent: &Symbol) -> Option<Symbol> {
    let syntax = &resolver.syntax[&parent.file_path];
    let file = &resolver.files[&parent.file_path];
    let bases: Vec<_> = file.inheritances.iter().filter(|i| i.source_key == parent.key).collect();
    if bases.len() != 1 || !plain_base(syntax, parent, &bases[0].target_name) {
        return None;
    }
    let base = bases[0];
    let call = Call {
        file_path: file.source.path.clone(),
        name: base.target_name.clone(),
        start_byte: parent.start_byte,
        end_byte: parent.end_byte,
        line: base.line,
        evidence: base.evidence.clone(),
        bare_target: true,
        qualifier: None,
        relationship: "CALLS".into(),
    };
    let request = ResolutionRequest {
        file,
        call: &call,
        owner: resolver.symbols.get(&parent.parent_key),
        name: base.target_name.clone(),
        qualifier: None,
    };
    let matches = syntax.adapter.inheritance(resolver, &request, resolver.named(&request, false));
    if matches.len() == 1 && matches[0].kind == "class" { Some(matches[0].clone()) } else { None }
}

fn instance_member_write(syntax: &Syntax, parent: &Symbol, name: &str) -> bool {
    receiver_scopes(syntax, parent).into_iter().any(|(nodes, receivers)| {
        nodes.into_iter().any(|node| {
            let target = text(syntax.adapter.write_target(node), &syntax.source);
            syntax.adapter.member_writes().contains(&node.kind())
                && !receivers.contains(target.trim())
                && member_write(&target, &receivers, name)
                || member_mutation(node, &receivers, &syntax.source)
        })
    })
}

fn receiver_scopes<'a>(syntax: &'a Syntax, parent: &Symbol) -> Vec<(Vec<Node<'a>>, BTreeSet<String>)> {
    if let Some(cached) = syntax.receiver_scope_cache.lock().unwrap().get(&parent.key) {
        return restore_receiver_scopes(syntax, cached);
    }
    let result = scan_receiver_scopes(syntax, parent);
    let stored = result
        .iter()
        .map(|(nodes, receivers)| (nodes.iter().map(|node| syntax.reference(*node)).collect(), receivers.clone()))
        .collect();
    syntax.receiver_scope_cache.lock().unwrap().insert(parent.key.clone(), stored);
    result
}
fn restore_receiver_scopes<'a>(
    syntax: &'a Syntax,
    scopes: &ReceiverScopeData,
) -> Vec<(Vec<Node<'a>>, BTreeSet<String>)> {
    scopes
        .iter()
        .map(|(nodes, receivers)| {
            (nodes.iter().filter_map(|node| syntax.dereference(node)).collect(), receivers.clone())
        })
        .collect()
}
fn scope_groups<'a>(syntax: &'a Syntax, parent: &Symbol) -> Vec<(Node<'a>, Vec<Node<'a>>)> {
    let mut indices = BTreeMap::new();
    let mut scopes: Vec<(Node<'a>, Vec<Node<'a>>)> = vec![];
    for node in
        syntax.nodes().into_iter().filter(|n| parent.start_byte <= n.start_byte() && n.start_byte() < parent.end_byte)
    {
        let scope = instance_scope(node, parent);
        let index = indices.entry(scope.id()).or_insert_with(|| {
            scopes.push((scope, vec![]));
            scopes.len() - 1
        });
        scopes[*index].1.push(node);
    }
    scopes
}
fn scan_receiver_scopes<'a>(syntax: &'a Syntax, parent: &Symbol) -> Vec<(Vec<Node<'a>>, BTreeSet<String>)> {
    scope_groups(syntax, parent)
        .into_iter()
        .map(|(scope, nodes)| {
            let mut receivers: BTreeSet<String> =
                syntax.adapter.self_qualifiers().iter().map(|s| (*s).to_owned()).collect();
            if syntax.adapter.family() == "python"
                && let Some(receiver) = method_receiver(scope, parent, &syntax.source)
            {
                receivers.insert(receiver);
            }
            let aliases = lexical_aliases(&nodes, receivers, &syntax.source);
            (nodes, aliases)
        })
        .collect()
}

fn instance_scope<'a>(mut node: Node<'a>, parent: &Symbol) -> Node<'a> {
    let mut scope = node;
    while node.parent().is_some() && node.start_byte() != parent.start_byte {
        if instance_function(node) {
            scope = node;
        }
        node = node.parent().unwrap();
    }
    if instance_function(scope) { scope } else { node }
}
fn instance_function(node: Node<'_>) -> bool {
    ["function_definition", "method_definition", "arrow_function", "function_expression", "function_declaration"]
        .contains(&node.kind())
}
fn method_receiver(node: Node<'_>, parent: &Symbol, source: &[u8]) -> Option<String> {
    if node.kind() != "function_definition" {
        return None;
    }
    let mut owner = node.parent();
    if owner.is_some_and(|n| n.kind() == "decorated_definition") {
        owner = owner.and_then(|n| n.parent());
    }
    if owner.and_then(|n| n.parent()).is_none_or(|n| n.start_byte() != parent.start_byte) {
        return None;
    }
    field(node, &["parameters"])
        .and_then(|n| n.named_child(0))
        .map(|n| text(Some(n), source).split(':').next().unwrap_or("").to_owned())
}

fn lexical_aliases(nodes: &[Node<'_>], mut receivers: BTreeSet<String>, source: &[u8]) -> BTreeSet<String> {
    loop {
        let aliases: BTreeSet<_> = nodes
            .iter()
            .filter(|n| {
                ["assignment", "named_expression", "assignment_expression", "variable_declarator"].contains(&n.kind())
                    && receivers.contains(&text(field(**n, &["right", "value"]), source))
            })
            .filter_map(|n| field(*n, &["left", "name"]).filter(|n| n.kind() == "identifier"))
            .map(|n| text(Some(n), source))
            .collect();
        if aliases.is_subset(&receivers) {
            return receivers;
        }
        receivers.extend(aliases);
    }
}

fn instance_field(syntax: &Syntax, parent: &Symbol, name: &str) -> bool {
    syntax.node(parent.start_byte, parent.end_byte).and_then(|n| field(n, &["body"])).is_some_and(|body| {
        named_children(body).into_iter().any(|child| {
            ["field_definition", "public_field_definition"].contains(&child.kind())
                && !children(child).iter().any(|c| c.kind() == "static")
                && field(child, &["name", "property"])
                    .is_some_and(|n| n.kind() == "computed_property_name" || syntax.text(n) == name)
        })
    })
}
fn plain_class(syntax: &Syntax, parent: &Symbol) -> bool {
    syntax
        .node(parent.start_byte, parent.end_byte)
        .is_some_and(|n| n.parent().is_none_or(|p| p.kind() != "decorated_definition"))
}

/// Detect supported Object/Reflect or Python attribute mutator calls on any known receiver.
/// Read function and first-argument spellings from matching `source`; reject unrelated node shapes.
pub fn member_mutation(node: Node<'_>, receivers: &BTreeSet<String>, source: &[u8]) -> bool {
    if !["call_expression", "call"].contains(&node.kind()) {
        return false;
    }
    let function = text(field(node, &["function"]), source);
    let mutators = [
        "Object.defineProperty",
        "Object.defineProperties",
        "Object.assign",
        "Object.setPrototypeOf",
        "Reflect.defineProperty",
        "Reflect.set",
        "Reflect.deleteProperty",
        "setattr",
        "delattr",
    ];
    mutators.contains(&function.as_str())
        && field(node, &["arguments"])
            .and_then(|n| n.named_child(0))
            .is_some_and(|n| receivers.contains(&text(Some(n), source)))
}

/// Test whether a target writes `name`, an indexed member, or a receiver itself.
/// Remove whitespace from `target` and compare against exact spellings in `receivers`.
pub fn member_write(target: &str, receivers: &BTreeSet<String>, name: &str) -> bool {
    let target: String = target.chars().filter(|c| !c.is_whitespace() && !('\u{1c}'..='\u{1f}').contains(c)).collect();
    receivers.iter().any(|r| target == format!("{r}.{name}") || target.starts_with(&format!("{r}[")) || target == *r)
}
fn plain_base(syntax: &Syntax, parent: &Symbol, target: &str) -> bool {
    if !plain_class(syntax, parent) {
        return false;
    }
    let Some(node) = syntax.node(parent.start_byte, parent.end_byte) else {
        return false;
    };
    let bases = field(node, &["superclasses"])
        .or_else(|| named_children(node).into_iter().find(|c| c.kind() == "class_heritage"));
    let Some(bases) = bases.filter(|n| n.named_child_count() == 1) else {
        return false;
    };
    let base = bases.named_child(0).unwrap();
    let base = if base.kind() == "extends_clause" { field(base, &["value"]) } else { Some(base) };
    base.is_some_and(|n| {
        let value = syntax.text(n);
        PLAIN_BASE.is_match(&value) && value == target
    })
}

/// Test whether nonempty candidates share the same file, parent key, and name.
pub fn same_declaration_group(matches: &[Symbol]) -> bool {
    let Some(first) = matches.first() else {
        return false;
    };
    matches.iter().all(|s| (&s.file_path, &s.parent_key, &s.name) == (&first.file_path, &first.parent_key, &first.name))
}

/// Return one body-bearing implementation in a shared declaration group.
/// Every other candidate must be a preceding signature of a kind in `signature_types`.
/// Missing syntax, unsupported signatures, and ambiguity yield no targets.
pub fn executable_implementation(
    identity: &CallableIdentity,
    matches: Vec<Symbol>,
    signature_types: &[&str],
) -> Vec<Symbol> {
    if !same_declaration_group(&matches) {
        return vec![];
    }
    let nodes: Option<Vec<_>> =
        matches.iter().map(|s| identity.syntax[&s.file_path].node(s.start_byte, s.end_byte)).collect();
    let Some(nodes) = nodes else {
        return vec![];
    };
    let bodies: Vec<_> =
        matches.iter().zip(&nodes).filter(|(_, n)| field(**n, &["body"]).is_some()).map(|(s, _)| s.clone()).collect();
    if bodies.len() != 1 {
        return vec![];
    }
    let valid = matches
        .iter()
        .zip(nodes)
        .filter(|(_, n)| field(*n, &["body"]).is_none())
        .all(|(s, n)| signature_types.contains(&n.kind()) && s.start_byte < bodies[0].start_byte);
    if valid { bodies } else { vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receiver_scope_cache_preserves_equal_span_identity_and_insertion_order() {
        let file = crate::parsing::parser::parse_file(crate::domain::models::SourceFile {
            path: "scope.py".into(),
            absolute_path: "/project/scope.py".into(),
            language: "python".into(),
            source: b"class C: pass".to_vec(),
        })
        .unwrap();
        let parent = file.symbols.iter().find(|s| s.kind == "class").unwrap();
        let syntax = Syntax::new(&file, super::super::adapter_for_language("python"));
        let groups = scope_groups(&syntax, parent);
        assert_eq!(groups.iter().map(|(scope, _)| scope.kind()).collect::<Vec<_>>(), ["module", "class_definition"]);
        assert_eq!(groups[0].0.byte_range(), groups[1].0.byte_range());
        let initial = receiver_scopes(&syntax, parent);
        assert_eq!(syntax.receiver_scope_cache.lock().unwrap().len(), 1);
        let cloned = syntax.clone();
        assert!(std::sync::Arc::ptr_eq(&syntax.receiver_scope_cache, &cloned.receiver_scope_cache));
        let restored = receiver_scopes(&cloned, parent);
        assert_eq!(initial.len(), restored.len());
        assert_eq!(restored[0].0[0].kind(), "module");
        assert_eq!(restored[1].0[0].kind(), "class_definition");
    }
    #[test]
    fn writes_include_index_and_receiver_replacement() {
        let receivers = BTreeSet::from(["self".to_owned()]);
        assert!(member_write("self . run", &receivers, "run"));
        assert!(member_write("self\u{1c}.run", &receivers, "run"));
        assert!(member_write("self[key]", &receivers, "run"));
        assert!(member_write("self", &receivers, "run"));
        assert!(!member_write("self.other", &receivers, "run"));
    }
}
