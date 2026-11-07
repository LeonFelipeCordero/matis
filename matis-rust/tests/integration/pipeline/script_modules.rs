//! Source and emitted-extension imports must preserve internal module and callable links.
use crate::support::{assert_json, fixtures, snapshot};
use matis::indexer::analyze_repository;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn source_and_emitted_extensions_keep_module_relationships_internal() {
    let graph = analyze_repository(&fixtures().join("script_modules")).unwrap();
    assert_eq!(graph["stats"]["parseErrors"], 0);
    let nodes: BTreeMap<_, _> =
        graph["nodes"].as_array().unwrap().iter().map(|node| (node["stableKey"].as_str().unwrap(), node)).collect();
    let mut rows: Vec<Value> = graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| matches!(edge["type"].as_str(), Some("IMPORTS" | "CALLS")))
        .map(|edge| {
            let source = nodes[edge["sourceStableKey"].as_str().unwrap()];
            let target = nodes[edge["targetStableKey"].as_str().unwrap()];
            assert_ne!(target["level"], "external", "unresolved relationship: {edge}");
            json!({"source": source["qualifiedName"], "target": target["qualifiedName"],
                "type": edge["type"], "typeOnly": edge["typeOnly"].as_bool().unwrap_or(false), "evidence": edge["evidence"]})
        })
        .collect();
    rows.sort_by_key(|row| ["source", "type", "evidence"].map(|key| row[key].as_str().unwrap().to_owned()));
    assert_json(&json!(rows), &snapshot("script_modules.json"), "");
}
