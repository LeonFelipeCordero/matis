//! Ordered graph emission, separate from repository-local binding indexes.
use super::super::resolution_contract::ImportRequest;
use super::*;
use crate::domain::models::{Import, Inheritance};
use anyhow::Result;
use std::collections::HashSet;

#[derive(Default)]
struct GraphBuilder {
    graph: ResolvedGraph,
    external_keys: HashSet<String>,
}
impl GraphBuilder {
    fn external(&mut self, language: &str, kind: &str, name: &str) -> String {
        let key = format!("external:{language}:{kind}:{name}");
        if self.external_keys.insert(key.clone()) {
            self.graph.external_nodes.push(ExternalNode {
                key: key.clone(),
                language: language.into(),
                kind: kind.into(),
                name: name.into(),
            });
        }
        key
    }
    fn import(&mut self, item: &Import, resolver: &CallResolver) {
        let file = &resolver.files[&item.file_path];
        let syntax = &resolver.syntax[&item.file_path];
        let directive = syntax.import_node(item.line, Some(&item.source));
        let adapter = &syntax.adapter;
        let request = ImportRequest {
            file,
            path: adapter.import_path(&item.source),
            position: directive.map(|n| n.start_byte()),
            source: item.source.clone(),
        };
        let targets = if adapter.require_import_node() && directive.is_none() {
            vec![(request.path.clone(), None)]
        } else {
            adapter.import_targets(&resolver.imports, &request)
        };
        for (name, key) in targets {
            let confidence = if key.is_some() { "INFERRED" } else { "EXTRACTED" };
            let key = key.unwrap_or_else(|| self.external(&file.source.language, "module", &name));
            self.graph.connections.push(connection(
                (file_key(&item.file_path), key),
                ("IMPORTS", confidence, adapter.import_type_only(directive)),
                (&item.file_path, item.line, &item.source),
            ));
        }
    }
    fn inheritance(&mut self, item: &Inheritance, resolver: &CallResolver) {
        let matches = inheritance_targets(resolver, item);
        let key = if matches.len() == 1 {
            matches[0].key.clone()
        } else {
            self.external(&item.language, "type", &item.target_name)
        };
        self.graph.connections.push(connection(
            (item.source_key.clone(), key),
            ("INHERITS", "INFERRED", false),
            (&item.file_path, item.line, &item.evidence),
        ));
    }
    fn call(&mut self, call: &Call, resolver: &CallResolver) {
        let owner = resolver.owner(call);
        let source = owner.map(|s| s.key.clone()).unwrap_or_else(|| file_key(&call.file_path));
        let matches = resolver.resolve(call, owner);
        let targets: Vec<_> = if matches.is_empty() {
            vec![self.external(&resolver.files[&call.file_path].source.language, "callable", &call.name)]
        } else {
            matches.into_iter().map(|s| s.key).collect()
        };
        self.graph.connections.extend(targets.into_iter().map(|target| {
            connection(
                (source.clone(), target),
                (&call.relationship, "INFERRED", false),
                (&call.file_path, call.line, &call.evidence),
            )
        }));
    }
}
/// Emit import, inheritance, and call connections in repository order.
/// Represent unresolved targets as deduplicated external nodes; return an in-memory graph.
/// This implementation does not return errors for unresolved relationships.
pub fn resolve_connections(repository: &ParsedRepository) -> Result<ResolvedGraph> {
    let resolver = CallResolver::new(repository);
    let mut builder = GraphBuilder::default();
    for item in repository.imports() {
        builder.import(item, &resolver);
    }
    for item in repository.inheritances() {
        builder.inheritance(item, &resolver);
    }
    for call in repository.calls() {
        builder.call(call, &resolver);
    }
    Ok(builder.graph)
}
fn connection(keys: (String, String), semantics: (&str, &str, bool), location: (&str, usize, &str)) -> Connection {
    let (source_key, target_key) = keys;
    let (type_, confidence, type_only) = semantics;
    let (path, line, evidence) = location;
    Connection {
        source_key,
        target_key,
        type_: type_.into(),
        confidence: confidence.into(),
        path: path.into(),
        line,
        evidence: Some(evidence.into()),
        type_only,
    }
}
fn inheritance_targets(resolver: &CallResolver, item: &Inheritance) -> Vec<Symbol> {
    let file = &resolver.files[&item.file_path];
    let declaration = &resolver.symbols[&item.source_key];
    let call = Call {
        file_path: file.source.path.clone(),
        name: item.target_name.clone(),
        start_byte: declaration.start_byte,
        end_byte: declaration.end_byte,
        line: item.line,
        evidence: item.evidence.clone(),
        bare_target: true,
        qualifier: None,
        relationship: "CALLS".into(),
    };
    let request = ResolutionRequest {
        file,
        call: &call,
        owner: resolver.symbols.get(&declaration.parent_key),
        name: item.target_name.clone(),
        qualifier: None,
    };
    let matches = resolver.named(&request, false);
    get_resolution_adapter(&file.source.language)
        .inheritance(resolver, &request, matches)
        .into_iter()
        .filter(|s| ["class", "interface", "struct", "trait", "type"].contains(&s.kind.as_str()))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_identity_is_deduplicated_without_reordering() {
        let mut builder = GraphBuilder::default();
        let first = builder.external("python", "callable", "f");
        builder.external("python", "module", "z");
        assert_eq!(builder.external("python", "callable", "f"), first);
        assert_eq!(builder.graph.external_nodes.len(), 2);
        assert_eq!(builder.graph.external_nodes[1].name, "z");
    }
}
