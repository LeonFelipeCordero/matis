//! TypeScript and TSX callable properties and signature implementation selection.
use super::resolution::CallableIdentity;
use super::{javascript, script_resolution::executable_implementation};
use crate::domain::models::Symbol;
use crate::metrics::resolution_contract::ResolutionRequest;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Count TypeScript inputs, including callable property signatures, excluding explicit `this`.
/// Use matching `source` through the script adapter; non-callable or unknown headers yield `None`.
pub fn parameter_count(node: Node<'_>, source: &[u8]) -> Option<usize> {
    let callable =
        if node.kind() == "property_signature" { crate::language_syntax::property_callable(node)? } else { node };
    javascript::parameter_count(callable, source)
}
/// Select an executable TypeScript implementation, excluding signature-only single candidates.
/// Multiple candidates need consistent staticness and one body following supported signatures.
/// The request argument is unused; unsafe or ambiguous groups yield no targets.
pub fn select(identity: &CallableIdentity, _request: &ResolutionRequest<'_>, matches: Vec<Symbol>) -> Vec<Symbol> {
    if matches.len() <= 1 {
        return matches
            .into_iter()
            .filter(|s| {
                identity.syntax[&s.file_path].node(s.start_byte, s.end_byte).is_some_and(|n| {
                    !["method_signature", "function_signature", "abstract_method_signature", "property_signature"]
                        .contains(&n.kind())
                })
            })
            .collect();
    }
    let statuses: BTreeSet<_> = matches.iter().map(|s| identity.syntax[&s.file_path].static_symbol(s)).collect();
    if statuses.len() != 1 {
        return vec![];
    }
    executable_implementation(identity, matches, &["method_signature", "function_signature"])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callable_property_inputs() {
        let source = b"interface API { run: (this: API, x: number, y?: string) => void; value: string }";
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&tree_sitter_language_pack::get_language("typescript").unwrap()).unwrap();
        let tree = parser.parse(source, None).unwrap();
        let properties: Vec<_> =
            crate::syntax::walk(tree.root_node()).filter(|n| n.kind() == "property_signature").collect();
        assert_eq!(parameter_count(properties[0], source), Some(2));
        assert_eq!(parameter_count(properties[1], source), None);
    }
}
