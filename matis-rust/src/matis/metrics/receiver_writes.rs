//! Receiver reassignment, alias propagation, and callable identity writes.
use std::collections::BTreeSet;
use tree_sitter::Node;

use super::context::ReceiverInference;
use crate::domain::models::Symbol;
use crate::metrics::receiver_syntax::ReceiverBinding;
use crate::metrics::resolution_contract::{FUNCTIONS, ResolutionRequest};
use crate::metrics::resolution_syntax::{Syntax, names};
use crate::syntax::{field, named_children, text};

impl ReceiverInference<'_> {
    /// Detect writes to a receiver binding or qualifier anywhere in its scope.
    /// Exempt the declaration and safe constructor initialization from a matching typed parameter.
    pub(super) fn reassigned(&self, syntax: &Syntax, binding: &ReceiverBinding<'_>, qualifier: Option<&str>) -> bool {
        let mut targets: BTreeSet<_> = [binding.name.clone()].into();
        if let Some(qualifier) = qualifier {
            targets.insert(qualifier.to_owned());
        }
        if binding.member {
            targets.extend([format!("self.{}", binding.name), format!("this.{}", binding.name)]);
        }
        syntax.nodes_in((binding.scope.start_byte(), binding.scope.end_byte())).into_iter().any(|n| {
            n != binding.node
                && Self::is_write(syntax, n)
                && !Self::write_names(syntax, n).is_disjoint(&targets)
                && !self.field_initialization(syntax, binding, n)
        })
    }

    fn is_write(syntax: &Syntax, node: Node<'_>) -> bool {
        syntax.adapter.assignments().contains(node.kind()) || syntax.adapter.receiver_updates().contains(node.kind())
    }

    fn field_initialization(&self, syntax: &Syntax, binding: &ReceiverBinding<'_>, node: Node<'_>) -> bool {
        if binding.inferred || !binding.member || !syntax.adapter.trust_receiver_annotations() {
            return false;
        }
        let mut parent = node.parent();
        while let Some(current) = parent.filter(|p| *p != binding.scope) {
            if current.kind() == "constructor_declaration" {
                return self.constructor_initialization(syntax, binding, (current, node));
            }
            parent = current.parent();
        }
        false
    }

    fn constructor_initialization(
        &self,
        syntax: &Syntax,
        binding: &ReceiverBinding<'_>,
        context: (Node<'_>, Node<'_>),
    ) -> bool {
        let (parent, node) = context;
        let value = text(field(node, &["right"]), &syntax.source);
        let entries = self.bindings_within(&syntax.file.source.path, &value, (parent.start_byte(), parent.end_byte()));
        let parameters: Vec<_> = entries.iter().filter(|b| !b.member && b.annotation == binding.annotation).collect();
        parameters.len() == 1 && !self.reassigned(syntax, parameters[0], Some(&value))
    }

    /// Detect alias-reachable writes to the requested member within the enclosing call scope.
    /// When `receiver` is true, also reject writes to the receiver itself; missing call syntax rejects.
    pub(super) fn path_written(&self, request: &ResolutionRequest<'_>, receiver: bool) -> bool {
        let syntax = &self.resolver.syntax[&request.file.source.path];
        let Some(call) = syntax.node(request.call.start_byte, request.call.end_byte) else {
            return true;
        };
        let scope = Self::call_scope(call);
        let nodes = syntax.nodes_in((scope.start_byte(), scope.end_byte()));
        let aliases: Vec<_> = nodes.iter().filter_map(|node| Self::node_alias(syntax, *node)).collect();
        let suffix = format!(".{}", request.name);
        let writes = nodes
            .iter()
            .filter(|node| Self::is_write(syntax, **node))
            .flat_map(|node| Self::write_names(syntax, *node));
        let targets = writes
            .flat_map(|write| {
                // Appending the member happens after alias expansion in the reference.
                // Equating whole member terms would incorrectly treat alias = x.run as a write to x.run.
                let parent = write.strip_suffix(&suffix).map(str::to_owned);
                parent.into_iter().chain(receiver.then_some(write))
            })
            .collect();
        super::aliases::reaches(&aliases, request.qualifier.as_deref().unwrap_or(""), &targets)
    }

    fn call_scope(mut call: Node<'_>) -> Node<'_> {
        while let Some(parent) = call.parent().filter(|_| !FUNCTIONS.contains(&call.kind())) {
            call = parent;
        }
        call
    }

    fn node_alias(syntax: &Syntax, node: Node<'_>) -> Option<(String, String)> {
        if !syntax.adapter.assignments().contains(node.kind()) && !syntax.adapter.declarations().contains(node.kind()) {
            return None;
        }
        let left = Self::single_expression(field(node, &["left", "name"]))?;
        let right = text(Self::single_expression(field(node, &["right", "value"])), &syntax.source).replace(' ', "");
        (left.kind() == "identifier" && !right.is_empty()).then(|| (syntax.text(left), right))
    }

    /// Test whether member writes in the request file replace the sole candidate type's member.
    /// Return false if `types` does not contain exactly one symbol.
    pub(super) fn identity_overwritten(&self, types: &[Symbol], request: &ResolutionRequest<'_>) -> bool {
        if types.len() != 1 {
            return false;
        }
        let syntax = &self.resolver.syntax[&request.file.source.path];
        syntax.nodes().into_iter().any(|node| self.identity_write(syntax, node, request).contains(&types[0].key))
    }

    fn identity_write(&self, syntax: &Syntax, node: Node<'_>, request: &ResolutionRequest<'_>) -> BTreeSet<String> {
        if !syntax.adapter.member_writes().contains(node.kind()) {
            return BTreeSet::new();
        }
        let target = text(syntax.adapter.write_target(node), &syntax.source).replace(' ', "");
        let (qualifier, member) = target.rsplit_once('.').unwrap_or(("", target.as_str()));
        if member != request.name {
            return BTreeSet::new();
        }
        let qualifier = qualifier.strip_suffix(".prototype").unwrap_or(qualifier);
        self.write_identities(node, qualifier, request).into_iter().map(|s| s.key).collect()
    }

    fn write_identities(&self, node: Node<'_>, qualifier: &str, request: &ResolutionRequest<'_>) -> Vec<Symbol> {
        let mut call = request.call.clone();
        call.start_byte = node.start_byte();
        call.end_byte = node.end_byte();
        let context = ResolutionRequest {
            call: &call,
            owner: self.resolver.owner(&call),
            name: qualifier.to_owned(),
            qualifier: None,
            ..request.clone()
        };
        let matches = self.resolver.named(&context, false);
        if matches.is_empty() { self.qualified_type(&context) } else { matches }
    }

    fn single_expression(node: Option<Node<'_>>) -> Option<Node<'_>> {
        let node = node?;
        let entries = named_children(node);
        if node.kind() == "expression_list" && entries.len() == 1 { Some(entries[0]) } else { Some(node) }
    }

    /// Collect binding names and the space-stripped full spelling of a write operand.
    pub(super) fn write_names(syntax: &Syntax, node: Node<'_>) -> BTreeSet<String> {
        let target = Self::single_expression(syntax.adapter.write_target(node));
        let mut result = names(target, &syntax.source);
        result.insert(text(target, &syntax.source).replace(' ', ""));
        result
    }
}
