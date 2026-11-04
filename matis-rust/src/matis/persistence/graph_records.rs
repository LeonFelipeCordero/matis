use super::{graph_inserts, models::*};
use anyhow::{Result, anyhow, ensure};
use sea_orm::ConnectionTrait;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub use super::graph_validation::{METRIC_NAMES, collection};
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
    super::graph_validation::validate_artifact(artifact)?;
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
/// Build an unsaved node row scoped to repository `id` and artifact `ordinal`.
/// Return an error for missing/non-string required node fields; optional strings become null.
pub fn node_record(id: &str, ordinal: usize, n: &Value) -> Result<GraphNodeRow> {
    Ok(GraphNodeRow {
        repository_id: id.into(),
        node_key: required(n, "stableKey")?,
        parent_key: text(n, "parentStableKey"),
        ordinal: ordinal as i64,
        type_: required(n, "type")?,
        kind: required(n, "kind")?,
        level: required(n, "level")?,
        name: required(n, "name")?,
        language: text(n, "language"),
        qualified_name: text(n, "qualifiedName"),
        path: text(n, "path"),
    })
}
fn symbol_record(id: &str, key: &str, n: &Value) -> GraphSymbolRow {
    GraphSymbolRow {
        repository_id: id.into(),
        node_key: key.into(),
        start_line: n["startLine"].as_i64(),
        start_column: n["startColumn"].as_i64(),
        end_line: n["endLine"].as_i64(),
        end_column: n["endColumn"].as_i64(),
        signature: text(n, "signature"),
        visibility: text(&n["properties"], "visibility"),
    }
}
fn metric_record(id: &str, key: &str, n: &Value) -> GraphNodeMetricRow {
    let p = &n["properties"];
    GraphNodeMetricRow {
        repository_id: id.into(),
        node_key: key.into(),
        line_count: p["lineCount"].as_i64(),
        parameter_count: p["parameterCount"].as_i64(),
        base_complexity: p["baseCyclomaticComplexity"].as_i64(),
        nested_complexity: p["nestedCyclomaticComplexity"].as_i64(),
        loop_complexity: p["loopNestedComplexity"].as_i64(),
        total_complexity: p["cyclomaticComplexity"].as_i64(),
        max_decision_depth: p["maxDecisionDepth"].as_i64(),
    }
}
/// Insert node details in foreign-key order with bounded batches.
/// Return construction/write errors; the caller owns the transaction and rollback.
pub async fn insert_node_details(connection: &impl ConnectionTrait, id: &str, artifact: &Value) -> Result<()> {
    let nodes = collection(artifact, "nodes");
    let files = nodes.iter().filter(|n| n["level"] == "file").map(|n| {
        Ok(GraphFileRow {
            repository_id: id.into(),
            node_key: required(n, "stableKey")?,
            byte_count: n["properties"]["bytes"].as_i64(),
            sha256: text(&n["properties"], "sha256"),
        })
    });
    graph_inserts::insert::<graph_files::Entity>(connection, files).await?;
    let symbols = nodes.iter().filter(|n| is_symbol(n));
    graph_inserts::insert::<graph_symbols::Entity>(
        connection,
        symbols.clone().map(|n| Ok(symbol_record(id, &required(n, "stableKey")?, n))),
    )
    .await?;
    let decorators = symbols.flat_map(|n| {
        collection(&n["properties"], "decorators").iter().enumerate().map(move |(ordinal, decorator)| {
            Ok(GraphNodeDecoratorRow {
                repository_id: id.into(),
                node_key: required(n, "stableKey")?,
                ordinal: ordinal as i64,
                decorator: decorator.as_str().ok_or_else(|| anyhow!("Graph decorators must contain strings"))?.into(),
            })
        })
    });
    graph_inserts::insert::<graph_node_decorators::Entity>(connection, decorators).await?;
    let metrics = nodes
        .iter()
        .filter(|n| METRIC_NAMES.iter().any(|name| n["properties"].get(name).is_some()))
        .map(|n| Ok(metric_record(id, &required(n, "stableKey")?, n)));
    graph_inserts::insert::<graph_node_metrics::Entity>(connection, metrics).await
}
/// Insert `artifact` edges for repository `id` in artifact order.
/// Return errors for required string fields or database failures; only literal `true`
/// sets `type_only`. The caller owns the transaction and any rollback.
pub async fn insert_edges(connection: &impl ConnectionTrait, id: &str, artifact: &Value) -> Result<()> {
    let rows = collection(artifact, "edges").iter().enumerate().map(|(ordinal, e)| {
        Ok(GraphEdgeRow {
            repository_id: id.into(),
            ordinal: ordinal as i64,
            source_key: required(e, "sourceStableKey")?,
            target_key: required(e, "targetStableKey")?,
            relationship: required(e, "type")?,
            confidence: required(e, "confidence")?,
            path: text(e, "path"),
            evidence: text(e, "evidence"),
            line: e["line"].as_i64(),
            type_only: e["typeOnly"] == true,
        })
    });
    graph_inserts::insert::<graph_edges::Entity>(connection, rows).await
}
/// Insert `artifact` diagnostics for repository `id` in artifact order.
/// Return errors for required string fields or failed writes; the caller owns the transaction.
pub async fn insert_diagnostics(connection: &impl ConnectionTrait, id: &str, artifact: &Value) -> Result<()> {
    let rows = collection(artifact, "diagnostics").iter().enumerate().map(|(ordinal, d)| {
        Ok(GraphDiagnosticRow {
            repository_id: id.into(),
            ordinal: ordinal as i64,
            severity: required(d, "severity")?,
            path: required(d, "path")?,
            language: required(d, "language")?,
            message: required(d, "message")?,
        })
    });
    graph_inserts::insert::<graph_diagnostics::Entity>(connection, rows).await
}
/// Insert ordered mutation candidates and JSON payloads for repository `id`.
/// Link each path to an artifact file node; fail for missing file matches, invalid
/// required fields, serialization, or database writes. The caller owns the transaction.
pub async fn insert_candidates(connection: &impl ConnectionTrait, id: &str, artifact: &Value) -> Result<()> {
    let files: HashMap<_, _> = collection(artifact, "nodes")
        .iter()
        .filter(|n| n["level"] == "file")
        .map(|n| Ok((required(n, "path")?, required(n, "stableKey")?)))
        .collect::<Result<_>>()?;
    let rows = collection(artifact, "mutationCandidates").iter().enumerate().map(|(ordinal, c)| {
        let path = required(c, "path")?;
        Ok(GraphMutationCandidateRow {
            repository_id: id.into(),
            candidate_id: required(c, "id")?,
            file_key: files.get(&path).ok_or_else(|| anyhow!("Candidate file not found: {path}"))?.clone(),
            ordinal: ordinal as i64,
            path,
            language: required(c, "language")?,
            source_hash: required(c, "sourceHash")?,
            kind: required(c, "kind")?,
            operator: text(c, "operator"),
            payload: serde_json::to_string(c)?,
        })
    });
    graph_inserts::insert::<graph_mutation_candidates::Entity>(connection, rows).await
}
