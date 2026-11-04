//! Language-neutral lexical module scopes. Stored scopes never borrow a tree.
use super::super::resolution_syntax::{NodeRef, Syntax};
use crate::{
    domain::models::{ParsedFile, Symbol, file_key},
    syntax::named_children,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::Arc,
};
use tree_sitter::Node;

#[derive(Clone)]
pub struct ModuleScope {
    pub file: Arc<ParsedFile>,
    pub identity: String,
    pub body: NodeRef,
    pub span: (usize, usize),
    pub directory: PathBuf,
}
impl ModuleScope {
    /// Restore this scope's body from its indexed syntax tree, or return `None` if unavailable.
    pub fn node<'a>(&self, syntax: &'a BTreeMap<String, Syntax>) -> Option<Node<'a>> {
        syntax.get(&self.file.source.path)?.dereference(&self.body)
    }
    /// Test whether the restored scope body is a file root with no parent.
    pub fn is_root(&self, syntax: &BTreeMap<String, Syntax>) -> bool {
        self.node(syntax).is_some_and(|n| n.parent().is_none())
    }
}

#[derive(Clone, Default)]
pub struct ModuleIndex {
    pub scopes: BTreeMap<String, Vec<Arc<ModuleScope>>>,
    pub file_scopes: BTreeMap<String, Vec<Arc<ModuleScope>>>,
}
impl ModuleIndex {
    /// Create empty indexes by module identity and file path.
    pub fn new() -> Self {
        Self::default()
    }
    /// Append owned scopes to both indexes without removing duplicates.
    pub fn add_scopes(&mut self, scopes: impl IntoIterator<Item = ModuleScope>) {
        for scope in scopes {
            let scope = Arc::new(scope);
            self.scopes.entry(scope.identity.clone()).or_default().push(Arc::clone(&scope));
            self.file_scopes.entry(scope.file.source.path.clone()).or_default().push(scope);
        }
    }
    /// Collect `root` and callback-provided descendants depth-first, using `syntax` for root checks.
    /// Avoid repeated file roots along each path; sibling traversal follows stack order.
    pub fn walk_scopes(
        root: ModuleScope,
        syntax: &BTreeMap<String, Syntax>,
        children: impl Fn(&ModuleScope) -> Vec<ModuleScope>,
    ) -> Vec<ModuleScope> {
        let seen = BTreeSet::from([root.file.source.path.clone()]);
        let mut pending = vec![(root, seen)];
        let mut result = Vec::new();
        while let Some((scope, seen)) = pending.pop() {
            pending.extend(pending_scopes(children(&scope), &seen, syntax));
            result.push(scope);
        }
        result
    }
    /// Return the narrowest scopes containing byte `position` in `source`, retaining ties.
    /// With no position, return only file-root scopes recognized through `syntax`.
    pub fn scopes_at<'a>(
        &'a self,
        source: &ParsedFile,
        position: Option<usize>,
        syntax: &BTreeMap<String, Syntax>,
    ) -> Vec<&'a ModuleScope> {
        let Some(scopes) = self.file_scopes.get(&source.source.path) else { return vec![] };
        let Some(position) = position else {
            return scopes.iter().map(Arc::as_ref).filter(|s| s.is_root(syntax)).collect();
        };
        let scopes: Vec<_> =
            scopes.iter().map(Arc::as_ref).filter(|s| s.span.0 <= position && position < s.span.1).collect();
        let width = scopes.iter().map(|s| s.span.1 - s.span.0).min();
        scopes.into_iter().filter(|s| Some(s.span.1 - s.span.0) == width).collect()
    }
    /// Resolve exactly one identity with exactly one indexed scope; otherwise return `None`.
    pub fn unique_scope(&self, identities: &[String]) -> Option<&ModuleScope> {
        if identities.len() != 1 {
            return None;
        }
        let scopes = self.scopes.get(&identities[0])?;
        if scopes.len() == 1 { scopes.first().map(Arc::as_ref) } else { None }
    }
    /// Return the scope's file key or enclosing declaration key, or `None` if unavailable.
    /// Restore the body from the corresponding tree in `syntax`.
    pub fn scope_key(scope: Option<&ModuleScope>, syntax: &BTreeMap<String, Syntax>) -> Option<String> {
        let scope = scope?;
        let body = scope.node(syntax)?;
        let Some(declaration) = body.parent() else {
            return Some(file_key(&scope.file.source.path));
        };
        scope
            .file
            .symbols
            .iter()
            .find(|s| s.start_byte == declaration.start_byte() && s.end_byte == declaration.end_byte())
            .map(|s| s.key.clone())
    }
    /// Return direct declarations of a file or inline scope using `syntax` to restore its body.
    /// Missing scopes or bodies produce no symbols.
    pub fn scope_symbols(scope: Option<&ModuleScope>, syntax: &BTreeMap<String, Syntax>) -> Vec<Symbol> {
        let Some(scope) = scope else {
            return vec![];
        };
        let Some(body) = scope.node(syntax) else {
            return vec![];
        };
        if body.parent().is_none() {
            return super::resolution_paths::top_symbols(&scope.file);
        }
        let spans: BTreeSet<_> = named_children(body).into_iter().map(|n| (n.start_byte(), n.end_byte())).collect();
        scope.file.symbols.iter().filter(|s| spans.contains(&(s.start_byte, s.end_byte))).cloned().collect()
    }
    /// Return the unique file for `identity`; inline, missing, or ambiguous scopes yield no files.
    pub fn scope_files(&self, identity: &str, syntax: &BTreeMap<String, Syntax>) -> Vec<ParsedFile> {
        self.unique_scope(&[identity.to_owned()])
            .filter(|s| s.is_root(syntax))
            .map(|s| vec![s.file.as_ref().clone()])
            .unwrap_or_default()
    }
}
fn pending_scopes(
    scopes: Vec<ModuleScope>,
    seen: &BTreeSet<String>,
    syntax: &BTreeMap<String, Syntax>,
) -> Vec<(ModuleScope, BTreeSet<String>)> {
    let mut result = Vec::new();
    for child in scopes {
        if child.is_root(syntax) && seen.contains(&child.file.source.path) {
            continue;
        }
        let mut child_seen = seen.clone();
        child_seen.insert(child.file.source.path.clone());
        result.push((child, child_seen));
    }
    result
}
