//! Exact projections from the original standalone safety/member/JVM snapshot helpers.
use crate::support;
use matis::{indexer::analyze_repository, parsing::discovery::discover_files};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use support::{assert_json, fixtures, snapshot};

fn nodes(graph: &Value) -> BTreeMap<&str, &Value> {
    graph["nodes"].as_array().unwrap().iter().map(|node| (node["stableKey"].as_str().unwrap(), node)).collect()
}

fn safety_rows(graph: &Value) -> Value {
    let nodes = nodes(graph);
    graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| edge["type"] == "CALLS")
        .map(|edge| {
            let source = nodes[edge["sourceStableKey"].as_str().unwrap()];
            let target = nodes[edge["targetStableKey"].as_str().unwrap()];
            json!({"source": source["qualifiedName"], "evidence": edge["evidence"], "target": target["qualifiedName"],
            "signature": target["signature"], "targetLine": target["startLine"]})
        })
        .collect()
}

fn member_rows(graph: &Value) -> Value {
    let nodes = nodes(graph);
    graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| matches!(edge["type"].as_str(), Some("CALLS" | "INHERITS")))
        .map(|edge| {
            let source = nodes[edge["sourceStableKey"].as_str().unwrap()];
            let target = nodes[edge["targetStableKey"].as_str().unwrap()];
            json!({"source": source["qualifiedName"], "evidence": edge["evidence"], "target": target["qualifiedName"],
            "targetLine": target["startLine"], "confidence": edge["confidence"]})
        })
        .collect()
}

fn selection_rows(graph: &Value) -> Value {
    let nodes = nodes(graph);
    graph["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| matches!(edge["type"].as_str(), Some("CALLS" | "REFERENCES")))
        .map(|edge| {
            let target = nodes[edge["targetStableKey"].as_str().unwrap()];
            json!({"type": edge["type"], "line": edge["line"], "evidence": edge["evidence"], "target": target["name"],
            "signature": target["signature"], "level": target["level"]})
        })
        .collect()
}

fn assert_project_seams(root: &Path, graph: &Value) {
    let paths: BTreeSet<_> = discover_files(root).unwrap().into_iter().map(|source| source.path).collect();
    assert!(!paths.is_empty());
    assert_eq!(graph["stats"]["parseErrors"], 0);
    let files: BTreeSet<_> = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["level"] == "file")
        .map(|node| node["path"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(files, paths, "every physical source must reach the artifact");
}

fn standalone(group: &str, language: &str) {
    let root = fixtures().join(group).join(language);
    let graph = analyze_repository(&root).unwrap();
    let actual = if group == "safety_review" { safety_rows(&graph) } else { member_rows(&graph) };
    assert_json(&actual, &snapshot(&format!("{group}/{language}.json")), "");
    assert_project_seams(&root, &graph);
}

fn callable_selection(language: &str) {
    let root = fixtures().join(language).join("callable_selection");
    let graph = analyze_repository(&root).unwrap();
    assert_json(&selection_rows(&graph), &snapshot(&format!("{language}_callable_selection.json")), "");
    assert_project_seams(&root, &graph);
}

macro_rules! family_tests {
    ($($language:ident),+ $(,)?) => {$ (
        mod $language {
            #[test] fn safety_review() { super::standalone("safety_review", stringify!($language)); }
        }
    )+};
}
family_tests!(java, kotlin, python, javascript, typescript);

#[test]
fn python_script_members() {
    standalone("script_members", "python");
}
#[test]
fn javascript_script_members() {
    standalone("script_members", "javascript");
}
#[test]
fn typescript_script_members() {
    standalone("script_members", "typescript");
}
#[test]
fn tsx_script_members() {
    standalone("script_members", "tsx");
}
#[test]
fn java_callable_selection() {
    callable_selection("java");
}
#[test]
fn kotlin_callable_selection() {
    callable_selection("kotlin");
}
