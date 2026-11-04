//! Full Python coffee pipeline records, not reduced artifact-derived substitutes.
mod serialization;
mod structure;
mod variants;
use matis::{
    metrics::{artifact::build_artifact, callables::add_callable_metrics, resolver::resolve_connections},
    parsing::{discovery::discover_files, parser::parse_files},
};
use serde_json::{Map, Value, json};
use std::path::Path;

pub struct Bundle {
    pub extraction: Value,
    pub analysis: Value,
    pub graph: Value,
}

pub fn project(root: &Path) -> Bundle {
    let sources = discover_files(root).unwrap();
    let mut repository = parse_files(sources.clone()).unwrap();
    let files: Value = repository.files.iter().map(serialization::parsed_file).collect();
    let structures: Map<_, _> =
        sources.iter().map(|source| (source.path.clone(), structure::source_structures(source))).collect();
    let extraction = json!({"files": files, "structures": structures, "excluded_sources": structure::excluded_sources(root),
        "grammar_variants": variants::grammar_variants(&sources)});
    add_callable_metrics(&mut repository).unwrap();
    let resolved = resolve_connections(&repository).unwrap();
    let symbols: Vec<_> = repository.symbols().collect();
    let analysis = json!({"symbols": symbols, "resolved": resolved});
    let mut graph = build_artifact(root, &repository, resolved).unwrap();
    graph["repository"].as_object_mut().unwrap().remove("indexedAt").expect("indexedAt before normalization");
    let canonical = format!("/samples/coffee/{}", root.file_name().unwrap().to_str().unwrap());
    graph["repository"]["path"] = json!(canonical);
    graph["nodes"][0]["properties"]["absolutePath"] = json!(canonical);
    Bundle { extraction, analysis, graph }
}
