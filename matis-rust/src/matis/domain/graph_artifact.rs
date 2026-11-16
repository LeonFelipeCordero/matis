pub use super::models::SOURCE_METRIC_NAMES as METRIC_NAMES;
use anyhow::{Result, anyhow, ensure};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
/// Test Python-style falseness for JSON null, false, zero, and empty containers/strings.
pub fn falsey(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(value) => !value,
        Value::Number(number) => number.as_f64() == Some(0.0),
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
    }
}
/// Accept an object or any Python-falsey value; otherwise return an error naming `name`.
pub fn optional_mapping(value: &Value, name: &str) -> Result<()> {
    ensure!(value.is_object() || falsey(value), "Graph {name} must be an object or empty");
    Ok(())
}

pub const POSITIONS: [&str; 4] = ["startLine", "startColumn", "endLine", "endColumn"];
/// Return an owned string field, or `None` when `name` is absent or not a string.
pub fn text(value: &Value, name: &str) -> Option<String> {
    value[name].as_str().map(str::to_owned)
}
/// Return string field `name`, or an error if it is absent or not a string.
/// Empty strings are accepted.
pub fn required(value: &Value, name: &str) -> Result<String> {
    text(value, name).ok_or_else(|| anyhow!("Missing or invalid graph field: {name}"))
}
/// Identify symbol-bearing nodes by level, source fields, or symbol properties.
/// Field presence counts even when its value is null; do not validate the node.
pub fn is_symbol(node: &Value) -> bool {
    let level = node["level"].as_str().unwrap_or("");
    matches!(level, "type" | "callable" | "symbol")
        || level == "module" && !matches!(node["kind"].as_str(), Some("directory" | "folder"))
        || node.get("signature").is_some()
        || POSITIONS.iter().any(|name| node.get(name).is_some())
        || node["properties"].get("visibility").is_some()
        || node["properties"].get("decorators").is_some()
}
/// Validate artifact shapes and reject missing/non-string node keys or parent cycles.
/// Leave missing parent references to database foreign-key checks; do not write rows.
pub fn validate_parentage(artifact: &Value) -> Result<()> {
    validate_artifact(artifact)?;
    let mut parents = HashMap::new();
    for node in collection(artifact, "nodes") {
        parents.insert(required(node, "stableKey")?, text(node, "parentStableKey"));
    }
    let mut complete = HashSet::new();
    for key in parents.keys() {
        validate_chain(key, &parents, &mut complete)?;
    }
    Ok(())
}
fn validate_chain(key: &str, parents: &HashMap<String, Option<String>>, complete: &mut HashSet<String>) -> Result<()> {
    let mut chain = HashSet::new();
    let mut current = Some(key);
    while let Some(key) = current.filter(|k| parents.contains_key(*k) && !complete.contains(*k)) {
        ensure!(chain.insert(key.to_owned()), "Graph node has cyclic parentage: {key}");
        current = parents[key].as_deref();
    }
    complete.extend(chain);
    Ok(())
}

/// Borrow array field `name`, or return an empty slice if it is absent or not an array.
/// This accessor does not validate collection shapes.
pub fn collection<'a>(value: &'a Value, name: &str) -> &'a [Value] {
    value[name].as_array().map(Vec::as_slice).unwrap_or(&[])
}

/// Check artifact objects, collection/record shapes, scalar fields, and decorator containers.
/// Return an error for invalid shapes without writing to storage. Required field types,
/// parent cycles, and database constraints need separate checks.
pub fn validate_artifact(artifact: &Value) -> Result<()> {
    ensure!(artifact.is_object(), "Graph artifact must be an object");
    optional_mapping(&artifact["repository"], "repository")?;
    scalar_fields(&artifact["repository"], &["name", "path", "indexedAt"])?;
    scalar_fields(artifact, &["schemaVersion"])?;
    for name in ["nodes", "edges", "diagnostics", "mutationCandidates"] {
        validate_records(artifact, name)?;
    }
    for node in collection(artifact, "nodes") {
        validate_node(node)?;
    }
    Ok(())
}
fn validate_records(artifact: &Value, name: &str) -> Result<()> {
    ensure!(artifact.get(name).is_none_or(Value::is_array), "Graph artifact collections must be arrays");
    for record in collection(artifact, name) {
        ensure!(record.is_object(), "Graph {name} records must be objects");
        scalar_fields(record, record_fields(name))?;
    }
    Ok(())
}
fn record_fields(name: &str) -> &'static [&'static str] {
    match name {
        "nodes" => &[
            "stableKey",
            "parentStableKey",
            "type",
            "kind",
            "level",
            "name",
            "language",
            "qualifiedName",
            "path",
            "signature",
            "startLine",
            "startColumn",
            "endLine",
            "endColumn",
        ],
        "edges" => &["sourceStableKey", "targetStableKey", "type", "confidence", "path", "evidence", "line"],
        "diagnostics" => &["severity", "path", "language", "message"],
        "mutationCandidates" => &["id", "path", "language", "sourceHash", "kind", "operator"],
        _ => unreachable!(),
    }
}
fn validate_node(node: &Value) -> Result<()> {
    let properties = &node["properties"];
    optional_mapping(properties, "properties")?;
    scalar_fields(properties, &METRIC_NAMES)?;
    scalar_fields(properties, &["visibility", "sha256", "bytes"])?;
    ensure!(
        properties.get("decorators").is_none_or(|value| value.is_null() || value.is_array()),
        "Graph decorators must be arrays or null"
    );
    Ok(())
}
fn scalar_fields(record: &Value, fields: &[&str]) -> Result<()> {
    for name in fields {
        ensure!(
            !matches!(record[*name], Value::Array(_) | Value::Object(_)),
            "Graph field {name} must be a SQLite scalar or null"
        );
    }
    Ok(())
}
