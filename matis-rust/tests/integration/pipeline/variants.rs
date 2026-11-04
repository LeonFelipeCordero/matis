//! Replay every explicit/shared-grammar variant from tests/coffee_pipeline.py.
use super::{serialization, structure::source_structures};
use matis::{
    domain::models::{ParsedRepository, SourceFile},
    metrics::{callables::add_callable_metrics, resolver::resolve_connections},
    parsing::parser::parse_files,
};
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

fn grammars(source: &SourceFile) -> &'static [&'static str] {
    match (source.language.as_str(), source.path.as_str()) {
        ("javascript", "ordering/Dispatch.js") => &["typescript", "tsx"],
        ("typescript", "ordering/Order.ts" | "ordering/Scheduling.ts") => &["tsx"],
        ("javascript", path) if path.ends_with(".jsx") => &["tsx"],
        _ => &[],
    }
}

fn resolution(repository: &ParsedRepository, path: &str) -> Value {
    let resolved = resolve_connections(repository).unwrap();
    let connections: Vec<_> = resolved.connections.iter().filter(|edge| edge.path == path).collect();
    let targets: BTreeSet<_> = connections.iter().map(|edge| &edge.target_key).collect();
    let external_nodes: Vec<_> = resolved.external_nodes.iter().filter(|node| targets.contains(&node.key)).collect();
    let file = repository.files.iter().find(|file| file.source.path == path).unwrap();
    json!({"symbols": file.symbols, "connections": connections, "external_nodes": external_nodes})
}

fn variant(source: &SourceFile, sources: &[SourceFile], grammar: &str) -> Value {
    let variant = SourceFile { language: grammar.to_owned(), ..source.clone() };
    let files =
        sources.iter().map(|item| if item.path == source.path { variant.clone() } else { item.clone() }).collect();
    let mut repository = parse_files(files).unwrap();
    let parsed =
        serialization::parsed_file(repository.files.iter().find(|file| file.source.path == source.path).unwrap());
    add_callable_metrics(&mut repository).unwrap();
    json!({"parser": parsed, "structures": source_structures(&variant), "analysis": resolution(&repository, &source.path)})
}

fn source_variants(source: &SourceFile, sources: &[SourceFile]) -> Value {
    grammars(source)
        .iter()
        .map(|grammar| ((*grammar).to_owned(), variant(source, sources, grammar)))
        .collect::<Map<_, _>>()
        .into()
}

pub fn grammar_variants(sources: &[SourceFile]) -> Value {
    sources
        .iter()
        .filter(|source| !grammars(source).is_empty())
        .map(|source| (source.path.clone(), source_variants(source, sources)))
        .collect::<Map<_, _>>()
        .into()
}
