//! Owned receiver binding descriptors. Tree paths preserve nodes with duplicate spans.
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use crate::metrics::receiver_syntax::ReceiverBinding;
use crate::metrics::resolution_syntax::NodeRef;
use crate::metrics::resolution_syntax::Syntax;

/// Owned binding index; borrowed syntax nodes never become resolver-owned data.
#[derive(Clone)]
pub struct ReceiverTypes {
    bindings: BTreeMap<String, Arc<FileBindings>>,
}

struct FileBindings {
    syntax: Syntax,
    entries: OnceLock<Vec<StoredReceiverBinding>>,
    names: OnceLock<BTreeMap<String, Vec<usize>>>,
}
impl FileBindings {
    fn named(&self, name: &str) -> impl Iterator<Item = &StoredReceiverBinding> {
        let entries = self.entries();
        self.names
            .get_or_init(|| {
                let mut names: BTreeMap<String, Vec<usize>> = BTreeMap::new();
                for (index, binding) in entries.iter().enumerate() {
                    names.entry(binding.name.clone()).or_default().push(index);
                }
                names
            })
            .get(name)
            .into_iter()
            .flatten()
            .map(move |index| &entries[*index])
    }
    fn entries(&self) -> &Vec<StoredReceiverBinding> {
        self.entries.get_or_init(|| {
            self.syntax
                .nodes()
                .into_iter()
                .flat_map(|node| self.syntax.adapter.receiver_bindings(&self.syntax, node))
                .map(|binding| StoredReceiverBinding::new(&self.syntax, binding))
                .collect()
        })
    }
}

#[derive(Clone)]
struct StoredReceiverBinding {
    name: String,
    annotation: Option<String>,
    scope: NodeRef,
    node: NodeRef,
    scope_span: (usize, usize),
    node_start: usize,
    inferred: bool,
    member: bool,
    static_: bool,
}

impl StoredReceiverBinding {
    fn new(syntax: &Syntax, binding: ReceiverBinding<'_>) -> Self {
        Self {
            name: binding.name,
            annotation: binding.annotation,
            scope: syntax.reference(binding.scope),
            node: syntax.reference(binding.node),
            scope_span: (binding.scope.start_byte(), binding.scope.end_byte()),
            node_start: binding.node.start_byte(),
            inferred: binding.inferred,
            member: binding.member,
            static_: binding.static_,
        }
    }

    fn restore<'tree>(&self, syntax: &'tree Syntax) -> Option<ReceiverBinding<'tree>> {
        Some(ReceiverBinding {
            name: self.name.clone(),
            annotation: self.annotation.clone(),
            scope: syntax.dereference(&self.scope)?,
            node: syntax.dereference(&self.node)?,
            inferred: self.inferred,
            member: self.member,
            static_: self.static_,
        })
    }
}
fn restore_bindings<'tree, 'stored>(
    syntax: &'tree Syntax,
    bindings: impl Iterator<Item = &'stored StoredReceiverBinding>,
) -> Vec<ReceiverBinding<'tree>> {
    bindings
        .map(|binding| binding.restore(syntax).expect("Receiver index must use the syntax trees it was built from"))
        .collect()
}

impl ReceiverTypes {
    /// Retain shared syntax clones per file and defer receiver extraction and name indexing.
    pub fn new(syntax: &BTreeMap<String, Syntax>) -> Self {
        let bindings = syntax
            .iter()
            .map(|(path, syntax)| {
                (
                    path.clone(),
                    Arc::new(FileBindings { syntax: syntax.clone(), entries: OnceLock::new(), names: OnceLock::new() }),
                )
            })
            .collect();
        Self { bindings }
    }

    /// Restore named bindings whose half-open scopes contain byte `position`.
    /// `syntax` must be the indexed file's tree or a shared clone.
    pub(super) fn bindings_at<'tree>(
        &self,
        syntax: &'tree Syntax,
        name: &str,
        position: usize,
    ) -> Vec<ReceiverBinding<'tree>> {
        restore_bindings(
            syntax,
            self.bindings[&syntax.file.source.path]
                .named(name)
                .filter(|binding| binding.scope_span.0 <= position && position < binding.scope_span.1),
        )
    }
    /// Restore named bindings with declaration starts in half-open `span`.
    /// `syntax` must be the indexed file's tree or a shared clone.
    pub(super) fn bindings_within<'tree>(
        &self,
        syntax: &'tree Syntax,
        name: &str,
        span: (usize, usize),
    ) -> Vec<ReceiverBinding<'tree>> {
        restore_bindings(
            syntax,
            self.bindings[&syntax.file.source.path]
                .named(name)
                .filter(|binding| span.0 <= binding.node_start && binding.node_start < span.1),
        )
    }
    #[cfg(test)]
    pub(super) fn bindings_for<'tree>(&self, syntax: &'tree Syntax) -> Vec<ReceiverBinding<'tree>> {
        self.bindings[&syntax.file.source.path]
            .entries()
            .iter()
            .map(|binding| binding.restore(syntax).expect("Receiver index must use the syntax trees it was built from"))
            .collect()
    }
}
