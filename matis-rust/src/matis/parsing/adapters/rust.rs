//! Rust calls include conservative parsing of known evaluating macro payloads.
use crate::{
    parsing::syntax::{
        self, CallSite, CallTarget, CallVisitor, DeclarationCarrier, callback_binding, callback_carrier, named_carrier,
    },
    syntax::{children, field, named_children, text, walk},
};
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;
pub const QUERY: &str = "\n(closure_expression) @definition.function\n(function_signature_item name: (identifier) @name) @definition.function\n";
pub const CALLBACK_TYPES: &[&str] = &["closure_expression"];
const EXPRESSION_MACROS: &[&str] = &[
    "println",
    "print",
    "eprintln",
    "eprint",
    "format",
    "format_args",
    "write",
    "writeln",
    "dbg",
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "vec",
    "panic",
];

/// Mark closures as callbacks with supported bindings; keep other declaration names.
/// `source` must correspond to the node's syntax tree.
pub fn declaration<'tree>(node: Node<'tree>, source: &[u8]) -> DeclarationCarrier<'tree> {
    if node.kind() == "closure_expression" {
        callback_carrier(node, callback_binding(node, source), source)
    } else {
        named_carrier(node, source)
    }
}

fn root(mut node: Node<'_>) -> Node<'_> {
    while let Some(parent) = node.parent() {
        node = parent;
    }
    node
}

fn word_present(value: &str, word: &str) -> bool {
    regex::Regex::new(&format!(r"(?:^|[^\p{{L}}\p{{N}}_]){}(?:$|[^\p{{L}}\p{{N}}_])", regex::escape(word)))
        .unwrap()
        .is_match(value)
}

fn qualified_macro_shadowed(node: Node<'_>, source: &[u8], namespace: &str) -> bool {
    if node.kind() == "mod_item" && text(field(node, &["name"]), source) == namespace {
        return true;
    }
    let value = text(Some(node), source);
    matches!(node.kind(), "use_declaration" | "extern_crate_declaration")
        && word_present(&value, namespace)
        && !value.starts_with(&format!("use {namespace}::"))
        && !value.starts_with(&format!("extern crate {namespace};"))
}

fn macro_shadowed(node: Node<'_>, source: &[u8], parts: &[&str]) -> bool {
    if parts.len() > 1 {
        return qualified_macro_shadowed(node, source, parts[0]);
    }
    let name = parts.last().copied().unwrap_or_default();
    let value = text(Some(node), source);
    node.kind() == "macro_definition" && text(field(node, &["name"]), source) == name
        || node.kind() == "use_declaration"
            && (value.contains(name) || value.contains('*'))
            && !value.starts_with("use std::")
            && !value.starts_with("use core::")
}

#[derive(Default)]
struct ShadowCache {
    roots: HashMap<usize, HashMap<String, bool>>,
    #[cfg(test)]
    scans: usize,
    #[cfg(test)]
    nodes: usize,
}

impl ShadowCache {
    fn shadowed(&mut self, (root, source): (Node<'_>, &[u8]), path: &str, parts: &[&str]) -> bool {
        let paths = self.roots.entry(root.id()).or_default();
        *paths.entry(path.to_owned()).or_insert_with(|| {
            #[cfg(test)]
            {
                self.scans += 1;
            }
            walk(root).any(|decl| {
                #[cfg(test)]
                {
                    self.nodes += 1;
                }
                macro_shadowed(decl, source, parts)
            })
        })
    }
}

fn evaluating(node: Node<'_>, traversal: &Traversal<'_>, cache: &mut ShadowCache) -> bool {
    let path = text(field(node, &["macro"]), traversal.source);
    let parts: Vec<_> = path.split("::").collect();
    let name = parts.last().copied().unwrap_or_default();
    if !EXPRESSION_MACROS.contains(&name) || parts.len() > 1 && !["std", "core"].contains(&parts[0]) {
        return false;
    }
    let current = root(node);
    let mut roots = vec![(current, traversal.source)];
    roots.extend_from_slice(&traversal.parents);
    if traversal.origin.0.id() != current.id() {
        roots.push(traversal.origin);
    }
    !roots.into_iter().any(|root| cache.shadowed(root, &path, &parts))
}

fn pattern_names(node: Node<'_>, source: &[u8]) -> HashSet<String> {
    if matches!(node.kind(), "identifier" | "field_identifier" | "shorthand_field_identifier") {
        return HashSet::from([text(Some(node), source)]);
    }
    children(node)
        .into_iter()
        .enumerate()
        .filter(|(index, child)| {
            child.is_named() && !matches!(node.field_name_for_child(*index as u32), Some("type" | "path"))
        })
        .flat_map(|(_, child)| pattern_names(child, source))
        .collect()
}

fn payload_binding(node: Node<'_>, source: &[u8], name: &str) -> bool {
    let pattern = match node.kind() {
        "let_declaration" | "let_condition" | "for_expression" | "match_arm" => field(node, &["pattern"]),
        "parameters" | "closure_parameters" => Some(node),
        _ => None,
    };
    let value = text(Some(node), source);
    if node.kind() == "use_declaration" && (word_present(&value, name) || value.contains('*')) {
        return true;
    }
    if matches!(node.kind(), "function_item" | "mod_item" | "struct_item" | "enum_item" | "const_item" | "static_item")
        && text(field(node, &["name"]), source) == name
    {
        return true;
    }
    pattern.is_some_and(|pattern| pattern_names(pattern, source).contains(name))
}

fn payload_unshadowed(site: &CallSite<'_>, source: &[u8], name: &str) -> bool {
    let mut target = field(site.node, &["function"]);
    while let Some(generic) = target.filter(|node| node.kind() == "generic_function") {
        target = field(generic, &["function"]).or_else(|| generic.named_child(0));
    }
    if target.is_some_and(|node| node.kind() == "field_expression") {
        return false;
    }
    let mut roots = vec![(root(site.node), source)];
    roots.extend_from_slice(&site.parents);
    !roots.into_iter().any(|(root, source)| walk(root).any(|decl| payload_binding(decl, source, name)))
}

/// Return the optional target name, bare-name flag, and qualifier using matching source bytes.
/// Preserve macro names with `!`; clear bare-name and qualifier evidence for synthetic
/// payload targets that have object receivers or potentially shadowing bindings.
pub fn normalize_call(site: &CallSite<'_>, source: &[u8]) -> CallTarget {
    if site.node.kind() == "macro_invocation" {
        return (Some(format!("{}!", text(field(site.node, &["macro"]), source))), false, None);
    }
    let (name, bare, qualifier) = syntax::normalize_call(site, source);
    let Some(value) = name.as_ref().filter(|s| !s.is_empty()) else {
        return (name, bare, qualifier);
    };
    let target = qualifier
        .as_deref()
        .unwrap_or(value)
        .split("::")
        .next()
        .unwrap_or_default()
        .split('.')
        .next()
        .unwrap_or_default();
    if site.opaque && !payload_unshadowed(site, source, target) {
        return (name, false, None);
    }
    (name, bare, qualifier)
}

#[derive(Clone)]
struct Traversal<'a> {
    root: Node<'a>,
    source: &'a [u8],
    origin: (Node<'a>, &'a [u8]),
    offset: isize,
    opaque: bool,
    parents: Vec<(Node<'a>, &'a [u8])>,
}

/// Invoke `visitor` for calls and macros, including recognized unshadowed evaluating macro payloads.
/// `source` must match `root`. Synthetic sites carry byte shifts and enclosing payload scopes;
/// the visitor receives the matching synthetic bytes. Skip payloads that cannot parse cleanly.
/// Parse payloads only; do not expand macros or execute source.
pub fn visit_calls<'a>(root: Node<'a>, source: &'a [u8], visitor: &mut CallVisitor<'_>) {
    visit(
        Traversal { root, source, origin: (root, source), offset: 0, opaque: false, parents: vec![] },
        visitor,
        &mut ShadowCache::default(),
    );
}

fn visit(traversal: Traversal<'_>, visitor: &mut CallVisitor<'_>, cache: &mut ShadowCache) {
    for node in walk(traversal.root) {
        if matches!(node.kind(), "call_expression" | "macro_invocation") {
            visitor(
                CallSite {
                    node,
                    offset: traversal.offset,
                    opaque: traversal.opaque,
                    parents: traversal.parents.clone(),
                },
                traversal.source,
            );
        }
        if node.kind() == "macro_invocation" {
            macro_sites((node, &traversal), visitor, cache);
        }
    }
    // Payload trees can reuse node IDs after they are dropped.
    cache.roots.remove(&root(traversal.root).id());
}

fn macro_sites((node, traversal): (Node<'_>, &Traversal<'_>), visitor: &mut CallVisitor<'_>, cache: &mut ShadowCache) {
    if !evaluating(node, traversal, cache) {
        return;
    }
    let tokens = named_children(node).into_iter().find(|child| child.kind() == "token_tree");
    let Some(tokens) = tokens.filter(|tokens| tokens.end_byte() - tokens.start_byte() >= 2) else {
        return;
    };
    let body = &traversal.source[tokens.start_byte() + 1..tokens.end_byte() - 1];
    for (prefix, suffix) in [("fn __macro_arguments(){ [", "]; }"), ("fn __macro_arguments(){ ", " }")] {
        let source = [prefix.as_bytes(), body, suffix.as_bytes()].concat();
        let Ok(tree) = syntax::parse("rust", &source) else {
            continue;
        };
        if tree.root_node().has_error() {
            continue;
        }
        let mut parents = traversal.parents.clone();
        if traversal.opaque {
            parents.insert(0, (traversal.root, traversal.source));
        }
        visit(
            Traversal {
                root: tree.root_node(),
                source: &source,
                origin: traversal.origin,
                offset: traversal.offset + tokens.start_byte() as isize + 1 - prefix.len() as isize,
                opaque: true,
                parents,
            },
            visitor,
            cache,
        );
        return;
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/parsing/adapters/rust_macro_tests.rs"]
mod macro_tests;
