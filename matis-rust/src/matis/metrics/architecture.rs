use super::artifact::projected_relationships;
use crate::paths::globs;
use serde_json::{Value, json};

pub(super) const RULE: &str = "forbiddenDependencies";

pub(super) fn findings(config: &Value, artifact: &Value) -> Vec<Value> {
    let rules = config[RULE].as_array().unwrap();
    if rules.is_empty() {
        return vec![];
    }
    let nodes = artifact["nodes"].as_array().unwrap();
    let edges = artifact["edges"].as_array().unwrap();
    let lookup = nodes.iter().map(|node| (node["stableKey"].as_str().unwrap(), node)).collect();
    let mut findings = Vec::new();
    for ((from, to), edges) in projected_relationships(edges, "file", &lookup) {
        let from = lookup[from.as_str()]["path"].as_str().unwrap();
        let to = lookup[to.as_str()]["path"].as_str().unwrap();
        findings.extend(
            rules
                .iter()
                .filter(|rule| matches(&rule["from"], from) && matches(&rule["to"], to))
                .map(|rule| finding(rule, (from, to), &edges)),
        );
    }
    findings
}

fn matches(patterns: &Value, path: &str) -> bool {
    patterns.as_array().unwrap().iter().any(|pattern| globs::matches(path, pattern.as_str().unwrap()))
}

fn finding(rule: &Value, (from, to): (&str, &str), edges: &[&Value]) -> Value {
    let mut source = json!({"path": from});
    if let Some(line) = edges.iter().filter(|edge| edge["path"] == from).filter_map(|edge| edge["line"].as_u64()).min()
    {
        source["line"] = json!(line);
    }
    json!({"rule": RULE, "metric": "dependencies", "subject": format!("{from} → {to}"), "actual": 1, "limit": 0,
        "files": [source, {"path": to}],
        "details": [rule["message"].as_str().unwrap_or("This direct dependency is forbidden.")]})
}
