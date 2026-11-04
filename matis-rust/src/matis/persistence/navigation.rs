use super::{
    database::{absolute_path, run},
    models::*,
    navigation_database::{ReadTransaction, readonly_session},
    navigation_queries::*,
};
use crate::domain::navigation::{EdgeRequest, GraphSnapshot, NavigationReader, error};
use anyhow::Result;
use sea_orm::{
    ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::{Expr, Func},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

fn node_query(id: &str) -> sea_orm::SelectTwo<graph_nodes::Entity, graph_symbols::Entity> {
    graph_nodes::Entity::find()
        .find_also_related(graph_symbols::Entity)
        .filter(graph_nodes::Column::RepositoryId.eq(id))
}
pub struct GraphReader {
    connection: ReadTransaction,
    pub snapshot: GraphSnapshot,
}
pub enum ProjectSelector<'a> {
    Name(&'a str),
    Directory(&'a Path),
}
/// Open one read-only graph snapshot at the supplied/default database path.
/// `project` selects an ID/name or the nearest containing local registration;
/// `revision`, when set, must match the saved revision. Never create or migrate storage.
///
/// # Errors
/// Reject absent/ambiguous projects, unindexed graphs, stale revisions, missing schema,
/// or path/database failures. The returned reader retains its transaction until dropped.
pub fn navigation_reader(
    project: ProjectSelector<'_>,
    revision: Option<&str>,
    path: Option<&Path>,
) -> Result<GraphReader> {
    let connection = readonly_session(path)?;
    let (id, location) = run(select_project(&connection, project))?;
    let graph = run(repository_graphs::Entity::find_by_id(&id).one(&*connection))?
        .filter(|g| g.graph_revision.is_some())
        .ok_or_else(|| error("Project has no normalized graph; index it first"))?;
    let snapshot = GraphSnapshot {
        repository_id: id,
        revision: graph.graph_revision.unwrap(),
        indexed_at: graph.indexed_at,
        root: location,
    };
    if let Some(expected) = revision.filter(|r| *r != snapshot.revision) {
        return Err(error(format!("Stale graph revision: expected {expected}, current {}", snapshot.revision)));
    }
    Ok(GraphReader { connection, snapshot })
}
async fn select_project(connection: &DatabaseTransaction, project: ProjectSelector<'_>) -> Result<(String, String)> {
    let rows = match project {
        ProjectSelector::Name(name) => repositories::Entity::find()
            .filter(
                Condition::any().add(repositories::Column::Id.eq(name)).add(super::repositories::name_matches(name)),
            )
            .order_by_asc(repositories::Column::Id)
            .limit(2)
            .all(connection)
            .await?
            .into_iter()
            .map(|r| (r.id, r.location))
            .collect(),
        ProjectSelector::Directory(directory) => containing_projects(connection, directory).await?,
    };
    if rows.len() != 1 {
        return Err(error("Project not found or ambiguous; use an explicit registered project ID with --project"));
    }
    Ok(rows.into_iter().next().unwrap())
}
async fn containing_projects(connection: &DatabaseTransaction, directory: &Path) -> Result<Vec<(String, String)>> {
    let current = absolute_path(directory)?;
    let mut matches = Vec::new();
    for (id, location) in repositories::Entity::find()
        .select_only()
        .columns([repositories::Column::Id, repositories::Column::Location])
        .into_tuple::<(String, String)>()
        .all(connection)
        .await?
    {
        if location.contains("://") {
            continue;
        }
        let root = absolute_path(Path::new(&location))?;
        if current.starts_with(&root) {
            matches.push((root.components().count(), id, location));
        }
    }
    if matches.is_empty() {
        return Err(error(
            "No indexed project contains the current directory; run matis index from the repository root or use --project",
        ));
    }
    let nearest = matches.iter().map(|m| m.0).max().unwrap();
    Ok(matches.into_iter().filter(|m| m.0 == nearest).map(|(_, id, location)| (id, location)).collect())
}
fn values(rows: Vec<(GraphNodeRow, Option<GraphSymbolRow>)>) -> Vec<Value> {
    rows.into_iter().map(|(n, s)| node_value(n, s)).collect()
}
impl NavigationReader for GraphReader {
    /// Borrow the identity and revision fixed when this read transaction opened.
    fn snapshot(&self) -> &GraphSnapshot {
        &self.snapshot
    }
    /// Read level/relation counts and up to 30 key-sorted roots with a truncation flag.
    /// Propagate query errors without modifying storage.
    fn overview(&mut self) -> Result<Value> {
        run(async {
            let id = &self.snapshot.repository_id;
            let levels: Vec<_> = graph_nodes::Entity::find()
                .filter(graph_nodes::Column::RepositoryId.eq(id))
                .select_only()
                .column(graph_nodes::Column::Level)
                .column_as(graph_nodes::Column::RepositoryId.count(), "count")
                .group_by(graph_nodes::Column::Level)
                .order_by_asc(graph_nodes::Column::Level)
                .into_tuple::<(String, i64)>()
                .all(&*self.connection)
                .await?
                .into_iter()
                .map(|(level, count)| json!({"level": level, "count": count}))
                .collect();
            let relations: Vec<_> = graph_edges::Entity::find()
                .filter(graph_edges::Column::RepositoryId.eq(id))
                .select_only()
                .column(graph_edges::Column::Relationship)
                .column_as(graph_edges::Column::RepositoryId.count(), "count")
                .group_by(graph_edges::Column::Relationship)
                .order_by_asc(graph_edges::Column::Relationship)
                .into_tuple::<(String, i64)>()
                .all(&*self.connection)
                .await?
                .into_iter()
                .map(|(relation, count)| json!({"type": relation, "count": count}))
                .collect();
            let mut roots = values(
                node_query(id)
                    .filter(graph_nodes::Column::ParentKey.is_null())
                    .order_by_asc(graph_nodes::Column::NodeKey)
                    .limit(31)
                    .all(&*self.connection)
                    .await?,
            );
            let truncated = roots.len() > 30;
            roots.truncate(30);
            Ok(json!({"levels": levels, "relations": relations, "roots": roots, "truncated": truncated}))
        })
    }
    /// Read up to `limit` ranked literal name/path matches or exact stable-key matches.
    /// Use SQLite case folding for names/paths, preferring exact keys and paths; propagate query errors.
    fn find(&mut self, query: &str, limit: usize) -> Result<Vec<Value>> {
        run(async {
            let matches = Condition::any().add(graph_nodes::Column::NodeKey.eq(query));
            let matches = [graph_nodes::Column::Path, graph_nodes::Column::Name, graph_nodes::Column::QualifiedName]
                .into_iter()
                .fold(matches, |condition, column| {
                    condition.add(Expr::cust_with_exprs(
                        "instr(lower(?),lower(?))>0",
                        [Expr::col((graph_nodes::Entity, column)).into(), Expr::val(query).into()],
                    ))
                });
            let rank = Expr::cust_with_values(
                "CASE WHEN graph_nodes.node_key=? THEN 0 WHEN lower(graph_nodes.path)=lower(?) THEN CASE WHEN graph_nodes.level='file' THEN 1 ELSE 2 END WHEN lower(graph_nodes.name)=lower(?) THEN 3 WHEN lower(graph_nodes.qualified_name)=lower(?) THEN 4 ELSE 5 END",
                [query, query, query, query],
            );
            Ok(values(
                node_query(&self.snapshot.repository_id)
                    .filter(matches)
                    .order_by(rank, sea_orm::Order::Asc)
                    .order_by(
                        Expr::col((graph_nodes::Entity, graph_nodes::Column::Level)).eq("external"),
                        sea_orm::Order::Asc,
                    )
                    .order_by(
                        Expr::expr(Func::coalesce([
                            Expr::col((graph_nodes::Entity, graph_nodes::Column::Path)).into(),
                            Expr::val("").into(),
                        ])),
                        sea_orm::Order::Asc,
                    )
                    .order_by_asc(graph_nodes::Column::Name)
                    .order_by_asc(graph_nodes::Column::NodeKey)
                    .limit(limit as u64)
                    .all(&*self.connection)
                    .await?,
            ))
        })
    }
    /// Read stable-key details with metrics, file hash, and up to 100 ordered decorators.
    /// Mark external nodes and decorator truncation; fail for absent keys or query errors.
    fn node(&mut self, key: &str) -> Result<Value> {
        run(async {
            let id = &self.snapshot.repository_id;
            let row = node_query(id)
                .filter(graph_nodes::Column::NodeKey.eq(key))
                .one(&*self.connection)
                .await?
                .ok_or_else(|| error(format!("Node not found: {key}. Use find to obtain a stableKey.")))?;
            let mut node = node_value(row.0, row.1);
            node["external"] = json!(node["level"] == "external");
            node["metrics"] = graph_node_metrics::Entity::find_by_id((id.clone(), key.to_owned()))
                .one(&*self.connection)
                .await?
                .map(metric_value)
                .unwrap_or(Value::Null);
            node["file"] = graph_files::Entity::find_by_id((id.clone(), key.to_owned()))
                .one(&*self.connection)
                .await?
                .map(|f| json!({"bytes": f.byte_count, "sha256": f.sha256}))
                .unwrap_or(Value::Null);
            let mut decorators = graph_node_decorators::Entity::find()
                .filter(graph_node_decorators::Column::RepositoryId.eq(id))
                .filter(graph_node_decorators::Column::NodeKey.eq(key))
                .select_only()
                .column(graph_node_decorators::Column::Decorator)
                .order_by_asc(graph_node_decorators::Column::Ordinal)
                .limit(101)
                .into_tuple::<String>()
                .all(&*self.connection)
                .await?;
            node["decoratorsTruncated"] = json!(decorators.len() > 100);
            decorators.truncate(100);
            node["decorators"] = json!(decorators);
            Ok(node)
        })
    }
    /// Read up to `limit` direct children of `key` after `offset`, in artifact order.
    /// Reject offsets outside SQLite's signed integer range; propagate query errors.
    /// An absent parent yields no rows rather than a missing-node error.
    fn children(&mut self, key: &str, limit: usize, offset: usize) -> Result<Vec<Value>> {
        run(async {
            Ok(values(
                node_query(&self.snapshot.repository_id)
                    .filter(graph_nodes::Column::ParentKey.eq(key))
                    .order_by_asc(graph_nodes::Column::Ordinal)
                    .order_by_asc(graph_nodes::Column::NodeKey)
                    .limit(limit as u64)
                    .offset(
                        i64::try_from(offset).map_err(|_| error("Python int too large to convert to SQLite INTEGER"))?
                            as u64,
                    )
                    .all(&*self.connection)
                    .await?,
            ))
        })
    }
    /// Read adjacency filtered by request direction/relations, capped at its limit.
    /// Deduplicate by ordinal and return artifact order; empty relations yield no rows.
    /// `out` selects outgoing, `both` selects both, and other directions select incoming;
    /// propagate query errors without validating the request.
    fn edges(&mut self, request: &EdgeRequest) -> Result<Vec<Value>> {
        run(async {
            let directions = match request.direction.as_str() {
                "both" => vec![true, false],
                "out" => vec![true],
                _ => vec![false],
            };
            let mut selected = BTreeMap::new();
            let batches = directions
                .into_iter()
                .flat_map(|outgoing| request.relations.iter().map(move |relation| (outgoing, relation)));
            for (outgoing, relation) in batches {
                selected.extend(
                    self.adjacency(request, outgoing, relation)
                        .await?
                        .into_iter()
                        .map(|edge| (edge.ordinal, edge_value(edge, true))),
                );
            }
            Ok(selected.into_values().take(request.limit).collect())
        })
    }
    /// Read diagnostics for exact `paths` in artifact order up to `limit`; propagate query errors.
    fn diagnostics(&mut self, paths: &[String], limit: usize) -> Result<Vec<Value>> {
        run(async {
            Ok(graph_diagnostics::Entity::find()
                .filter(graph_diagnostics::Column::RepositoryId.eq(&self.snapshot.repository_id))
                .filter(graph_diagnostics::Column::Path.is_in(paths.iter().cloned()))
                .order_by_asc(graph_diagnostics::Column::Ordinal)
                .limit(limit as u64)
                .all(&*self.connection)
                .await?
                .into_iter()
                .map(diagnostic_value)
                .collect())
        })
    }
    /// Read up to two key/hash records for exact `path`, sorted by key to detect ambiguity.
    /// Propagate query errors; do not access local source files.
    fn file(&mut self, path: &str) -> Result<Vec<Value>> {
        run(async {
            Ok(graph_nodes::Entity::find()
                .find_also_related(graph_files::Entity)
                .filter(graph_nodes::Column::RepositoryId.eq(&self.snapshot.repository_id))
                .filter(graph_nodes::Column::Path.eq(path))
                .filter(graph_files::Column::NodeKey.is_not_null())
                .order_by_asc(graph_nodes::Column::NodeKey)
                .limit(2)
                .all(&*self.connection)
                .await?
                .into_iter()
                .map(|(n, f)| json!({"stableKey": n.node_key, "sha256": f.and_then(|f| f.sha256)}))
                .collect())
        })
    }
}
impl GraphReader {
    async fn adjacency(&mut self, request: &EdgeRequest, outgoing: bool, relation: &str) -> Result<Vec<GraphEdgeRow>> {
        let mut query = graph_edges::Entity::find()
            .filter(graph_edges::Column::RepositoryId.eq(&self.snapshot.repository_id))
            .filter(graph_edges::Column::Relationship.eq(relation));
        query = if outgoing {
            query.filter(graph_edges::Column::SourceKey.eq(&request.key))
        } else {
            query.filter(graph_edges::Column::TargetKey.eq(&request.key))
        };
        Ok(query.order_by_asc(graph_edges::Column::Ordinal).limit(request.limit as u64).all(&*self.connection).await?)
    }
}
