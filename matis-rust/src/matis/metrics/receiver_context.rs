//! Shared receiver lookup context, independent of inference and write detection.
use crate::domain::models::Symbol;
use crate::metrics::receiver_syntax::ReceiverBinding;
use crate::metrics::resolution_contract::ResolutionRequest;
use crate::metrics::resolver::CallResolver;

pub(super) struct ReceiverInference<'resolver> {
    pub(super) resolver: &'resolver CallResolver,
}

impl<'resolver> ReceiverInference<'resolver> {
    /// Borrow the resolver and its shared receiver indexes for one inference context.
    pub(super) fn new(resolver: &'resolver CallResolver) -> Self {
        Self { resolver }
    }

    /// Find receiver bindings named `name` in `path` whose scopes contain byte `position`.
    pub(super) fn bindings_at(&self, path: &str, name: &str, position: usize) -> Vec<ReceiverBinding<'resolver>> {
        self.resolver.receivers.bindings_at(&self.resolver.syntax[path], name, position)
    }
    /// Find receiver bindings named `name` in `path` with declaration starts in half-open `span`.
    pub(super) fn bindings_within(
        &self,
        path: &str,
        name: &str,
        span: (usize, usize),
    ) -> Vec<ReceiverBinding<'resolver>> {
        self.resolver.receivers.bindings_within(&self.resolver.syntax[path], name, span)
    }
    #[cfg(test)]
    pub(super) fn bindings(&self, path: &str) -> Vec<ReceiverBinding<'resolver>> {
        self.resolver.receivers.bindings_for(&self.resolver.syntax[path])
    }

    /// Split a dotted or `::` type name and resolve it as a qualified request.
    /// Return no candidates if the name has no nonempty qualifier.
    pub(super) fn qualified_type(&self, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let separator = if request.name.contains("::") { "::" } else { "." };
        let Some((qualifier, name)) = request.name.rsplit_once(separator).filter(|(q, _)| !q.is_empty()) else {
            return vec![];
        };
        self.resolver.qualified(&ResolutionRequest {
            name: name.to_owned(),
            qualifier: Some(qualifier.to_owned()),
            ..request.clone()
        })
    }
}
