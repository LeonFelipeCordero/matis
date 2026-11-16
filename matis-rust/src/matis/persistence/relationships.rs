use super::{
    database::run,
    graph_inserts,
    models::{GraphEdgeRow, graph_edges},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::RelationshipPort};
use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;

use super::navigation_queries::edge_value;
use crate::domain::navigation::{AdjacencyFilter, EdgeRequest};
pub(crate) struct Store(pub Session);

impl RelationshipPort for Store {
    fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                Ok(graph_edges::Entity::find()
                    .filter(graph_edges::Column::RepositoryId.eq(id))
                    .filter(graph_edges::Column::Relationship.ne("CONTAINS"))
                    .order_by_asc(graph_edges::Column::Ordinal)
                    .all(c)
                    .await?
                    .into_iter()
                    .map(|e| edge_value(e, false))
                    .collect())
            })
        })
    }
    fn counts(&self, id: &str) -> Result<Vec<(String, i64)>> {
        self.0.with(|c| {
            run(async {
                Ok(graph_edges::Entity::find()
                    .filter(graph_edges::Column::RepositoryId.eq(id))
                    .select_only()
                    .column(graph_edges::Column::Relationship)
                    .column_as(graph_edges::Column::RepositoryId.count(), "count")
                    .group_by(graph_edges::Column::Relationship)
                    .order_by_asc(graph_edges::Column::Relationship)
                    .into_tuple::<(String, i64)>()
                    .all(c)
                    .await?)
            })
        })
    }
    fn adjacency(&self, id: &str, request: &EdgeRequest, filter: AdjacencyFilter<'_>) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                let query = graph_edges::Entity::find()
                    .filter(graph_edges::Column::RepositoryId.eq(id))
                    .filter(graph_edges::Column::Relationship.eq(filter.relation));
                let query = if filter.outgoing {
                    query.filter(graph_edges::Column::SourceKey.eq(&request.key))
                } else {
                    query.filter(graph_edges::Column::TargetKey.eq(&request.key))
                };
                Ok(query
                    .order_by_asc(graph_edges::Column::Ordinal)
                    .limit(request.limit as u64)
                    .all(c)
                    .await?
                    .into_iter()
                    .map(|e| edge_value(e, true))
                    .collect())
            })
        })
    }
    fn clear(&self, id: &str) -> Result<()> {
        self.0.with(|c| {
            run(async {
                graph_edges::Entity::delete_many().filter(graph_edges::Column::RepositoryId.eq(id)).exec(c).await?;
                Ok(())
            })
        })
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
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
        self.0.with(|c| run(graph_inserts::insert::<graph_edges::Entity>(c, rows)))
    }
}
