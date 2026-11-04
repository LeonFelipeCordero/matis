use super::{models::*, navigation_queries::*};
use crate::domain::repositories::{GraphDto, GraphStatsDto};
use anyhow::Result;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Read repository `id`'s graph metadata, returning `None` if absent or unrevisioned.
/// Propagate database errors; do not start or commit a transaction.
pub async fn graph_row(connection: &impl ConnectionTrait, id: &str) -> Result<Option<RepositoryGraphRow>> {
    Ok(repository_graphs::Entity::find_by_id(id).one(connection).await?.filter(|g| g.graph_revision.is_some()))
}
/// Combine `row` metadata with stored node, symbol, edge, and parse-error counts.
/// Propagate query errors; the caller supplies a transaction for a consistent revision.
pub async fn graph_summary(connection: &impl ConnectionTrait, row: RepositoryGraphRow) -> Result<GraphDto> {
    let id = &row.repository_id;
    let levels: HashMap<String, i64> = graph_nodes::Entity::find()
        .filter(graph_nodes::Column::RepositoryId.eq(id))
        .select_only()
        .column(graph_nodes::Column::Level)
        .column_as(graph_nodes::Column::NodeKey.count(), "count")
        .group_by(graph_nodes::Column::Level)
        .into_tuple::<(String, i64)>()
        .all(connection)
        .await?
        .into_iter()
        .collect();
    let level = |name: &str| *levels.get(name).unwrap_or(&0);
    let symbols =
        graph_symbols::Entity::find().filter(graph_symbols::Column::RepositoryId.eq(id)).count(connection).await?
            as i64;
    let edges =
        graph_edges::Entity::find().filter(graph_edges::Column::RepositoryId.eq(id)).count(connection).await? as i64;
    let parse_errors = graph_diagnostics::Entity::find()
        .filter(graph_diagnostics::Column::RepositoryId.eq(id))
        .filter(graph_diagnostics::Column::Severity.eq("error"))
        .count(connection)
        .await? as i64;
    Ok(GraphDto {
        repository_id: row.repository_id,
        graph_revision: row.graph_revision.unwrap_or_default(),
        schema_version: row.schema_version,
        indexed_at: row.indexed_at,
        root_name: row.root_name,
        root_path: row.root_path,
        imported_at: row.imported_at,
        stats: GraphStatsDto {
            files: level("file"),
            symbols,
            modules: level("module"),
            types: level("type"),
            callables: level("callable"),
            nodes: levels.values().sum(),
            edges,
            external_nodes: level("external"),
            parse_errors,
        },
    })
}
/// Read ordered nodes, non-containment edges, diagnostics, and saved mutations for `row`.
/// Include an imports subset and graph revision. Return query or candidate-JSON errors;
/// the caller supplies a transaction for consistency. Do not read local sources.
pub async fn report_facts(connection: &impl ConnectionTrait, row: RepositoryGraphRow) -> Result<Value> {
    let id = &row.repository_id;
    let metrics: HashMap<_, _> = graph_node_metrics::Entity::find()
        .filter(graph_node_metrics::Column::RepositoryId.eq(id))
        .all(connection)
        .await?
        .into_iter()
        .map(|m| (m.node_key.clone(), metric_value(m)))
        .collect();
    let nodes = graph_nodes::Entity::find()
        .find_also_related(graph_symbols::Entity)
        .filter(graph_nodes::Column::RepositoryId.eq(id))
        .order_by_asc(graph_nodes::Column::Ordinal)
        .all(connection)
        .await?
        .into_iter()
        .map(|(n, s)| report_node(n, s, &metrics))
        .collect::<Vec<_>>();
    let edges: Vec<Value> = graph_edges::Entity::find()
        .filter(graph_edges::Column::RepositoryId.eq(id))
        .filter(graph_edges::Column::Relationship.ne("CONTAINS"))
        .order_by_asc(graph_edges::Column::Ordinal)
        .all(connection)
        .await?
        .into_iter()
        .map(|e| edge_value(e, false))
        .collect();
    let imports: Vec<_> = edges.iter().filter(|e| e["type"] == "IMPORTS").cloned().collect();
    let diagnostics: Vec<_> = graph_diagnostics::Entity::find()
        .filter(graph_diagnostics::Column::RepositoryId.eq(id))
        .order_by_asc(graph_diagnostics::Column::Ordinal)
        .all(connection)
        .await?
        .into_iter()
        .map(diagnostic_value)
        .collect();
    let candidates = graph_mutation_candidates::Entity::find()
        .filter(graph_mutation_candidates::Column::RepositoryId.eq(id))
        .order_by_asc(graph_mutation_candidates::Column::Ordinal)
        .select_only()
        .column(graph_mutation_candidates::Column::Payload)
        .into_tuple::<String>()
        .all(connection)
        .await?
        .into_iter()
        .map(|s| serde_json::from_str::<Value>(&s))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(json!({"graphRevision": row.graph_revision, "nodes": nodes, "edges": edges, "imports": imports,
        "diagnostics": diagnostics, "mutationCandidates": candidates}))
}
fn report_node(n: GraphNodeRow, symbol: Option<GraphSymbolRow>, metrics: &HashMap<String, Value>) -> Value {
    let metric = metrics.get(&n.node_key).unwrap_or(&Value::Null);
    let mut value = node_value(n, symbol);
    for field in ["language", "startColumn", "endLine", "endColumn", "signature", "visibility"] {
        value.as_object_mut().unwrap().remove(field);
    }
    for name in super::graph_records::METRIC_NAMES {
        value[name] = metric[name].clone();
    }
    value
}
