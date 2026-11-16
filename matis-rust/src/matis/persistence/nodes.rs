use super::{
    database::run,
    graph_inserts,
    models::{GraphNodeRow, graph_nodes},
    session::Session,
};
use crate::domain::{graph_artifact::*, navigation::ChildrenRequest, storage::NodePort};
use anyhow::Result;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    sea_query::{Expr, Func},
};
use serde_json::Value;

use super::navigation_queries::node_value;
pub(crate) struct Store(pub Session);

fn node_record(id: &str, ordinal: usize, n: &Value) -> Result<GraphNodeRow> {
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

fn node_query(id: &str) -> sea_orm::Select<graph_nodes::Entity> {
    graph_nodes::Entity::find().filter(graph_nodes::Column::RepositoryId.eq(id))
}
fn values(rows: Vec<GraphNodeRow>) -> Vec<Value> {
    rows.into_iter().map(|n| node_value(n, None)).collect()
}
impl NodePort for Store {
    fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async { Ok(values(node_query(id).order_by_asc(graph_nodes::Column::Ordinal).all(c).await?)) })
        })
    }
    fn get(&self, id: &str, key: &str) -> Result<Option<Value>> {
        self.0.with(|c| {
            run(async {
                Ok(node_query(id)
                    .filter(graph_nodes::Column::NodeKey.eq(key))
                    .one(c)
                    .await?
                    .map(|n| node_value(n, None)))
            })
        })
    }
    fn roots(&self, id: &str, limit: usize) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                Ok(values(
                    node_query(id)
                        .filter(graph_nodes::Column::ParentKey.is_null())
                        .order_by_asc(graph_nodes::Column::NodeKey)
                        .limit(limit as u64)
                        .all(c)
                        .await?,
                ))
            })
        })
    }
    fn find(&self, id: &str, query: &str, limit: usize) -> Result<Vec<Value>> {
        self.0.with(|c| { run(async {
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
                node_query(id)
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
                    .all(c)
                    .await?,
            ))
        }) })
    }
    fn children(&self, id: &str, request: &ChildrenRequest<'_>) -> Result<Vec<Value>> {
        let offset = i64::try_from(request.offset)
            .map_err(|_| crate::domain::navigation::error("Python int too large to convert to SQLite INTEGER"))?;
        self.0.with(|c| {
            run(async {
                Ok(values(
                    node_query(id)
                        .filter(graph_nodes::Column::ParentKey.eq(request.key))
                        .order_by_asc(graph_nodes::Column::Ordinal)
                        .order_by_asc(graph_nodes::Column::NodeKey)
                        .limit(request.limit as u64)
                        .offset(offset as u64)
                        .all(c)
                        .await?,
                ))
            })
        })
    }
    fn files_at(&self, id: &str, path: &str, limit: usize) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                Ok(values(
                    node_query(id)
                        .filter(graph_nodes::Column::Path.eq(path))
                        .filter(graph_nodes::Column::Level.eq("file"))
                        .order_by_asc(graph_nodes::Column::NodeKey)
                        .limit(limit as u64)
                        .all(c)
                        .await?,
                ))
            })
        })
    }
    fn levels(&self, id: &str) -> Result<Vec<(String, i64)>> {
        self.0.with(|c| {
            run(async {
                Ok(node_query(id)
                    .select_only()
                    .column(graph_nodes::Column::Level)
                    .column_as(graph_nodes::Column::NodeKey.count(), "count")
                    .group_by(graph_nodes::Column::Level)
                    .order_by_asc(graph_nodes::Column::Level)
                    .into_tuple::<(String, i64)>()
                    .all(c)
                    .await?)
            })
        })
    }
    fn clear(&self, id: &str) -> Result<()> {
        self.0.with(|c| {
            run(async {
                graph_nodes::Entity::delete_many().filter(graph_nodes::Column::RepositoryId.eq(id)).exec(c).await?;
                Ok(())
            })
        })
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        self.0.with(|c| {
            run(graph_inserts::insert::<graph_nodes::Entity>(
                c,
                collection(artifact, "nodes").iter().enumerate().map(|(o, n)| node_record(id, o, n)),
            ))
        })
    }
}
