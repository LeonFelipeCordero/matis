//! Resolve lexical receiver bindings without member-name or global type guesses.
use super::receiver_syntax::ReceiverBinding;
use super::resolution_contract::ResolutionRequest;
use super::resolution_syntax::Syntax;
use super::resolver::CallResolver;
use crate::domain::models::Symbol;
use crate::syntax::{field, text};

#[path = "receiver_aliases.rs"]
mod aliases;
#[path = "receiver_context.rs"]
mod context;
#[path = "receiver_index.rs"]
mod index;
#[path = "receiver_writes.rs"]
mod writes;

use context::ReceiverInference;
pub use index::ReceiverTypes;

impl ReceiverTypes {
    /// Resolve the request's receiver types, then find its instance member candidates.
    /// `None` delegates to other lookup; `Some([])` rejects unsafe or unresolved typed lookup.
    pub fn resolve(resolver: &CallResolver, request: &ResolutionRequest<'_>) -> Option<Vec<Symbol>> {
        let inference = ReceiverInference::new(resolver);
        inference.receiver_types(request).map(|types| resolver.members(&types, &request.name, false))
    }

    /// Infer receiver type symbols from lexical declarations and supported field paths.
    /// `None` means no applicable inference; an empty vector means inference rejects resolution.
    pub fn infer(resolver: &CallResolver, request: &ResolutionRequest<'_>) -> Option<Vec<Symbol>> {
        ReceiverInference::new(resolver).receiver_types(request)
    }
}

impl ReceiverInference<'_> {
    fn receiver_types(&self, request: &ResolutionRequest<'_>) -> Option<Vec<Symbol>> {
        let qualifier = request.qualifier.as_deref().unwrap_or("");
        let syntax = &self.resolver.syntax[&request.file.source.path];
        let root = qualifier.split('.').next().unwrap_or("");
        let implicit = syntax.adapter.self_qualifiers().contains(root);
        if implicit && let Some(types) = self.implicit_types(request, root) {
            return Some(types);
        }
        if qualifier.matches('.').count() > 1 || qualifier.contains('.') && !implicit {
            return self.field_types(request);
        }
        self.lexical_types(request, qualifier, syntax)
    }

    fn implicit_types(&self, request: &ResolutionRequest<'_>, root: &str) -> Option<Vec<Symbol>> {
        if self.path_written(request, true) {
            return Some(vec![]);
        }
        let syntax = &self.resolver.syntax[&request.file.source.path];
        let types = syntax.adapter.receiver_owner(self.resolver, request);
        if types.is_none() || request.qualifier.as_deref() == Some(root) {
            return types;
        }
        let name = request.qualifier.as_deref().unwrap_or("").split_once('.').map(|(_, name)| name).unwrap_or("");
        self.fields(types, name, request)
    }

    fn lexical_types(&self, request: &ResolutionRequest<'_>, qualifier: &str, syntax: &Syntax) -> Option<Vec<Symbol>> {
        let member = self.member_receiver(syntax, request);
        let root = qualifier.split('.').next().unwrap_or("");
        let context = ResolutionRequest { qualifier: Some(root.to_owned()), ..request.clone() };
        if member && self.self_class(&context).is_empty() {
            return Some(vec![]);
        }
        let name = if member { qualifier.split('.').next_back().unwrap_or("") } else { qualifier };
        if name.is_empty() || name.contains('.') || name.contains("::") {
            return None;
        }
        let bindings = self.bindings_at(&request.file.source.path, name, request.call.start_byte);
        let candidates: Vec<_> = bindings.iter().filter(|b| self.visible(syntax, b, request)).collect();
        if candidates.is_empty() {
            return None;
        }
        Some(self.binding_types(syntax, &Self::nearest(candidates), request))
    }

    fn binding_types(
        &self,
        syntax: &Syntax,
        candidates: &[&ReceiverBinding<'_>],
        request: &ResolutionRequest<'_>,
    ) -> Vec<Symbol> {
        if candidates.len() != 1 {
            return vec![];
        }
        let binding = candidates[0];
        if !self.member_receiver(syntax, request) && self.shadowed(syntax, binding, request.call.start_byte) {
            return vec![];
        }
        if self.unsafe_binding(syntax, binding, request) || self.path_written(request, false) {
            return vec![];
        }
        self.declared_types(syntax, binding, request)
    }

    fn nearest<'a, 'tree>(candidates: Vec<&'a ReceiverBinding<'tree>>) -> Vec<&'a ReceiverBinding<'tree>> {
        let size = candidates.iter().map(|b| b.scope.end_byte() - b.scope.start_byte()).min().unwrap();
        let candidates: Vec<_> =
            candidates.into_iter().filter(|b| b.scope.end_byte() - b.scope.start_byte() == size).collect();
        let declared: Vec<_> = candidates.iter().copied().filter(|b| !b.inferred).collect();
        if declared.is_empty() { candidates } else { declared }
    }

    fn member_receiver(&self, syntax: &Syntax, request: &ResolutionRequest<'_>) -> bool {
        let qualifier = request.qualifier.as_deref().unwrap_or("");
        qualifier.contains('.') && syntax.adapter.self_qualifiers().contains(qualifier.split('.').next().unwrap_or(""))
    }

    fn visible(&self, syntax: &Syntax, binding: &ReceiverBinding<'_>, request: &ResolutionRequest<'_>) -> bool {
        let position = request.call.start_byte;
        if self.member_receiver(syntax, request) {
            return binding.member && binding.scope.start_byte() <= position && position < binding.scope.end_byte();
        }
        if binding.member && !syntax.adapter.bare_instance_fields() {
            return false;
        }
        syntax.visible(binding.scope, position)
    }

    fn unsafe_binding(&self, syntax: &Syntax, binding: &ReceiverBinding<'_>, request: &ResolutionRequest<'_>) -> bool {
        binding.member && !binding.static_ && self.static_context(request)
            || self.conditional(syntax, binding)
            || binding.annotation.as_ref().is_none_or(|s| s.is_empty())
            || self.reassigned(syntax, binding, request.qualifier.as_deref())
            || !binding.member && binding.node.start_byte() >= request.call.start_byte
    }

    fn conditional(&self, syntax: &Syntax, binding: &ReceiverBinding<'_>) -> bool {
        if !binding.inferred && syntax.adapter.trust_receiver_annotations() {
            return false;
        }
        let mut node = binding.node.parent();
        while let Some(current) = node.filter(|n| *n != binding.scope) {
            if matches!(
                current.kind(),
                "if_statement"
                    | "for_statement"
                    | "while_statement"
                    | "try_statement"
                    | "if_expression"
                    | "when_expression"
                    | "conditional_expression"
            ) {
                return true;
            }
            node = current.parent();
        }
        false
    }

    fn declared_types(
        &self,
        syntax: &Syntax,
        binding: &ReceiverBinding<'_>,
        request: &ResolutionRequest<'_>,
    ) -> Vec<Symbol> {
        let Some(annotation) = binding.annotation.as_deref().filter(|s| !s.is_empty()) else {
            return vec![];
        };
        if !syntax.adapter.receiver_type_allowed(syntax, annotation, &request.name) {
            return vec![];
        }
        let mut call = request.call.clone();
        call.file_path = syntax.file.source.path.clone();
        call.start_byte = binding.node.start_byte();
        call.end_byte = binding.node.end_byte();
        let context = ResolutionRequest {
            file: &syntax.file,
            call: &call,
            owner: self.resolver.owner(&call),
            name: annotation.to_owned(),
            qualifier: None,
        };
        let mut types = self.resolver.named(&context, false);
        types = syntax.adapter.qualify_class(self.resolver, &context, types);
        if types.is_empty() {
            types = self.qualified_type(&context);
        }
        let Some(types) = Self::type_symbols(types) else {
            return vec![];
        };
        if let Some(initialized) = self.initializer_types(syntax, binding, request)
            && Self::conflicting_types(&types, &initialized)
        {
            return vec![];
        }
        if self.identity_overwritten(&types, request) { vec![] } else { types }
    }

    fn type_symbols(types: Vec<Symbol>) -> Option<Vec<Symbol>> {
        const KINDS: &[&str] = &["class", "interface", "struct", "trait", "type", "enum", "object"];
        if types.iter().any(|s| !KINDS.contains(&s.kind.as_str()) && s.kind != "impl") {
            return None;
        }
        Some(types.into_iter().filter(|s| KINDS.contains(&s.kind.as_str())).collect())
    }

    fn conflicting_types(types: &[Symbol], initialized: &[Symbol]) -> bool {
        types.len() != 1 || initialized.len() != 1 || types[0].key != initialized[0].key
    }

    fn initializer_types(
        &self,
        syntax: &Syntax,
        binding: &ReceiverBinding<'_>,
        request: &ResolutionRequest<'_>,
    ) -> Option<Vec<Symbol>> {
        if binding.inferred || !syntax.adapter.validate_receiver_initializer() {
            return None;
        }
        self.value_types(syntax, binding, request)
    }

    fn value_types(
        &self,
        syntax: &Syntax,
        binding: &ReceiverBinding<'_>,
        request: &ResolutionRequest<'_>,
    ) -> Option<Vec<Symbol>> {
        let value = field(binding.node, &["right", "value"])?;
        if value.kind() == "identifier" {
            let mut call = request.call.clone();
            call.file_path = syntax.file.source.path.clone();
            call.start_byte = binding.node.start_byte();
            call.end_byte = binding.node.end_byte();
            let context = ResolutionRequest {
                file: &syntax.file,
                call: &call,
                owner: self.resolver.owner(&call),
                qualifier: Some(syntax.text(value)),
                name: request.name.clone(),
            };
            return Some(self.receiver_types(&context).unwrap_or_default());
        }
        if syntax.adapter.receiver_class_nodes().contains(value.kind()) {
            let (callee, receiver) = syntax.adapter.call_target(Some(value));
            let annotation = if receiver.is_none() {
                text(callee, &syntax.source)
            } else {
                format!("{}.{}", text(receiver, &syntax.source), text(callee, &syntax.source))
            };
            let binding = ReceiverBinding { annotation: Some(annotation), inferred: true, ..binding.clone() };
            return Some(self.declared_types(syntax, &binding, request));
        }
        Some(vec![])
    }

    fn field_types(&self, request: &ResolutionRequest<'_>) -> Option<Vec<Symbol>> {
        let (qualifier, name) = request.qualifier.as_deref().unwrap_or("").rsplit_once('.').unwrap_or(("", ""));
        let types =
            self.receiver_types(&ResolutionRequest { qualifier: Some(qualifier.to_owned()), ..request.clone() });
        self.fields(types, name, request)
    }

    fn fields(&self, types: Option<Vec<Symbol>>, name: &str, request: &ResolutionRequest<'_>) -> Option<Vec<Symbol>> {
        let types = types?;
        if types.len() != 1 {
            return Some(vec![]);
        }
        let parent = &types[0];
        let entries = self.bindings_within(&parent.file_path, name, (parent.start_byte, parent.end_byte));
        let bindings: Vec<_> = entries.iter().filter(|binding| binding.member).collect();
        if bindings.len() != 1 {
            return Some(vec![]);
        }
        let syntax = &self.resolver.syntax[&parent.file_path];
        if self.path_written(request, true)
            || self.conditional(syntax, bindings[0])
            || self.reassigned(syntax, bindings[0], request.qualifier.as_deref())
        {
            return Some(vec![]);
        }
        Some(self.declared_types(syntax, bindings[0], request))
    }

    fn static_context(&self, request: &ResolutionRequest<'_>) -> bool {
        self.resolver.ancestors(request.owner).iter().any(|s| {
            matches!(s.kind.as_str(), "method" | "function") && self.resolver.syntax[&s.file_path].static_symbol(s)
        })
    }

    fn self_class(&self, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let ancestors = self.resolver.ancestors(request.owner);
        let Some(parent) = ancestors.iter().find(|s| matches!(s.kind.as_str(), "class" | "struct" | "enum" | "object"))
        else {
            return vec![];
        };
        let Some(method) = ancestors.iter().find(|s| s.parent_key == parent.key) else {
            return vec![];
        };
        let syntax = &self.resolver.syntax[&request.file.source.path];
        if syntax.static_symbol(method) || !syntax.adapter.implicit_receiver(self.resolver, request, method) {
            return vec![];
        }
        vec![parent.clone()]
    }

    fn shadowed(&self, syntax: &Syntax, binding: &ReceiverBinding<'_>, position: usize) -> bool {
        syntax.bindings_at(&binding.name, position).into_iter().any(|(scope, node)| {
            scope.end_byte() - scope.start_byte() <= binding.scope.end_byte() - binding.scope.start_byte()
                && !(node.start_byte() <= binding.node.start_byte() && binding.node.start_byte() < node.end_byte())
                && !syntax.adapter.assignments().contains(node.kind())
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/metrics/receiver_tests.rs"]
mod tests;
