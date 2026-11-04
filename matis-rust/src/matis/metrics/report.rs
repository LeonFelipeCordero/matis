pub use super::report_markdown::render_markdown;
use super::{
    artifact::{LEVELS, build_dependency_views},
    cycles::circular_dependencies,
    dependency_metrics::dependency_metrics,
};
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const COMPLEXITY_METRICS: [(&str, &str); 5] = [
    ("baseCyclomaticComplexity", "cyclomaticComplexity"),
    ("cyclomaticComplexity", "weightedComplexity"),
    ("nestedCyclomaticComplexity", "nestedDecisionPenalty"),
    ("loopNestedComplexity", "nestedLoops"),
    ("maxDecisionDepth", "maxDecisionDepth"),
];

fn is_callable(node: &Value) -> bool {
    node["kind"] == "function" || node["kind"] == "method"
}
fn key(node: &Value) -> &str {
    node["stableKey"].as_str().unwrap_or("")
}

fn file_owner<'a>(node: &'a Value, lookup: &BTreeMap<&str, &'a Value>) -> Option<&'a str> {
    let mut current = node;
    let mut visited = BTreeSet::new();
    while current["kind"] != "file" {
        let parent = current["parentStableKey"].as_str()?;
        if !visited.insert(parent) {
            return None;
        }
        current = lookup.get(parent)?;
    }
    Some(key(current))
}

fn file_summaries(nodes: &[Value]) -> Result<BTreeMap<String, Value>> {
    let mut summaries: BTreeMap<_, _> = nodes
        .iter()
        .filter(|node| node["kind"] == "file")
        .map(|node| {
            (key(node).to_owned(), json!({"functionCount": 0, "measuredFunctionCount": 0, "weightedComplexitySum": 0}))
        })
        .collect();
    let lookup = nodes.iter().map(|node| (key(node), node)).collect();
    for node in nodes.iter().filter(|node| is_callable(node)) {
        let Some(owner) = file_owner(node, &lookup) else {
            continue;
        };
        let values = summaries.get_mut(owner).ok_or_else(|| anyhow!("missing file owner: {owner}"))?;
        values["functionCount"] = json!(values["functionCount"].as_u64().unwrap() + 1);
        if !node["properties"]["cyclomaticComplexity"].is_null() {
            let complexity = node["properties"]["cyclomaticComplexity"]
                .as_i64()
                .ok_or_else(|| anyhow!("invalid cyclomaticComplexity"))?;
            values["measuredFunctionCount"] = json!(values["measuredFunctionCount"].as_u64().unwrap() + 1);
            values["weightedComplexitySum"] = json!(values["weightedComplexitySum"].as_i64().unwrap() + complexity);
        }
    }
    Ok(summaries)
}

fn measurement(node: &Value, dependencies: &BTreeMap<String, Value>, files: &BTreeMap<String, Value>) -> Value {
    let properties = &node["properties"];
    let mut metrics = json!({});
    if node["kind"] == "file" || is_callable(node) {
        metrics["lineCount"] = properties["lineCount"].clone();
    }
    if is_callable(node) {
        metrics["parameterCount"] = properties["parameterCount"].clone();
        for (name, label) in COMPLEXITY_METRICS {
            metrics[label] = properties[name].clone();
        }
    }
    for values in [files.get(key(node)), dependencies.get(key(node))].into_iter().flatten() {
        metrics.as_object_mut().unwrap().extend(values.as_object().unwrap().clone());
    }
    let subject = node["qualifiedName"]
        .as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| node["name"].as_str().filter(|s| !s.is_empty()))
        .unwrap_or("Unnamed item");
    json!({"stableKey": node["stableKey"], "subject": subject, "kind": node["kind"], "level": node["level"],
        "path": node["path"], "line": node["startLine"], "metrics": metrics})
}

fn cycle_measurements(groups: &[Value], nodes: &[Value]) -> Result<Vec<Value>> {
    let paths: BTreeMap<_, _> = nodes
        .iter()
        .map(|node| (key(node), node["path"].as_str().filter(|s| !s.is_empty()).unwrap_or("Unknown file")))
        .collect();
    groups.iter().map(|group| {
        let members = group["nodeStableKeys"].as_array().ok_or_else(|| anyhow!("cycle members must be an array"))?;
        let edges = group["edges"].as_array().ok_or_else(|| anyhow!("cycle edges must be an array"))?;
        let pairs: BTreeSet<_> = edges.iter().map(|edge| (edge["sourceStableKey"].as_str(), edge["targetStableKey"].as_str())).collect();
        let path = |value: &Value| paths.get(value.as_str().unwrap_or("")).copied().unwrap_or("Unknown file");
        let mut files: Vec<_> = members.iter().map(path).collect();
        files.sort();
        let example: Vec<_> = group["exampleCycle"].as_array().ok_or_else(|| anyhow!("exampleCycle must be an array"))?.iter().map(path).collect();
        Ok(json!({"id": group["id"], "groupSize": members.len(), "internalDependencyCount": pairs.len(), "files": files, "exampleCycle": example}))
    }).collect()
}

/// Build ordered measurements, file summaries, and cycle summaries from an artifact.
/// Recompute dependency views and detect cycles if the stored cycle section is absent or empty.
/// Preserve diagnostics and mutation candidates without modifying `artifact`.
///
/// # Errors
/// Reject malformed arrays, cycle data, diagnostics, or measured complexity values.
pub fn build_artifact_report(artifact: &Value) -> Result<Value> {
    let empty = Vec::new();
    let nodes = optional_array(artifact, "nodes")?.unwrap_or(&empty);
    let edges = optional_array(artifact, "edges")?.unwrap_or(&empty);
    let detected;
    let cycles = match artifact
        .get("circularDependencies")
        .filter(|value| !value.is_null() && value.as_object().is_none_or(|o| !o.is_empty()))
    {
        Some(value) => value,
        None => {
            detected = circular_dependencies(nodes, edges);
            &detected
        }
    };
    let groups = cycles["groups"].as_array().ok_or_else(|| anyhow!("circularDependencies.groups must be an array"))?;
    let views = build_dependency_views(nodes, edges);
    let dependencies = LEVELS.iter().flat_map(|level| dependency_metrics(&views[level])).collect();
    let files = file_summaries(nodes)?;
    let count = |level: &str| nodes.iter().filter(|node| node["level"] == level).count();
    let diagnostics = artifact.get("diagnostics").cloned().unwrap_or_else(|| json!([]));
    let mut ordered: Vec<_> = nodes.iter().collect();
    ordered
        .sort_by_key(|node| (node["path"].as_str().unwrap_or(""), node["startLine"].as_u64().unwrap_or(0), key(node)));
    let measurements: Vec<_> = ordered
        .into_iter()
        .filter(|node| node["level"].as_str().is_some_and(|level| LEVELS.contains(&level)) || is_callable(node))
        .map(|node| measurement(node, &dependencies, &files))
        .collect();
    Ok(
        json!({"summary": {"files": count("file"), "modules": count("module"), "types": count("type"), "callables": count("callable"),
        "circularImportGroups": groups.len(), "parseErrors": diagnostics.as_array().ok_or_else(|| anyhow!("diagnostics must be an array"))?.len()},
        "nodes": measurements, "circularImports": cycle_measurements(groups, nodes)?, "diagnostics": diagnostics,
        "mutationCandidates": artifact.get("mutationCandidates").cloned().unwrap_or_else(|| json!([]))}),
    )
}

fn optional_array<'a>(value: &'a Value, field: &str) -> Result<Option<&'a Vec<Value>>> {
    value.get(field).map(|value| value.as_array().ok_or_else(|| anyhow!("{field} must be an array"))).transpose()
}
fn record_node(node: &Value) -> Value {
    let mut result = node.clone();
    let mut properties = json!({});
    for name in COMPLEXITY_METRICS.iter().map(|(name, _)| *name).chain(["lineCount", "parameterCount"]) {
        properties[name] = node[name].clone();
    }
    result["properties"] = properties;
    result
}

/// Normalize saved graph facts with flat metrics and return an artifact-style report.
/// Prefer nonempty `edges` over legacy `imports`; preserve diagnostics and mutation candidates.
///
/// # Errors
/// Return an error if nodes or normalized report data are malformed.
pub fn build_report(graph: &Value) -> Result<Value> {
    let nodes: Vec<_> =
        graph["nodes"].as_array().ok_or_else(|| anyhow!("nodes must be an array"))?.iter().map(record_node).collect();
    let edges = graph
        .get("edges")
        .filter(|value| value.as_array().is_some_and(|edges| !edges.is_empty()))
        .or_else(|| graph.get("imports"))
        .cloned()
        .unwrap_or_else(|| json!([]));
    build_artifact_report(&json!({"nodes": nodes, "edges": edges,
        "diagnostics": graph.get("diagnostics").cloned().unwrap_or_else(|| json!([])),
        "mutationCandidates": graph.get("mutationCandidates").cloned().unwrap_or_else(|| json!([]))}))
}
