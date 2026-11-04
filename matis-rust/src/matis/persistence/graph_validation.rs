use super::graph_metadata::optional_mapping;
pub use crate::domain::models::SOURCE_METRIC_NAMES as METRIC_NAMES;
use anyhow::{Result, ensure};
use serde_json::Value;
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
