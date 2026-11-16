use super::{
    database::run,
    graph_inserts,
    models::{GraphDiagnosticRow, graph_diagnostics},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::DiagnosticPort};
use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;

use super::navigation_queries::diagnostic_value;
pub(crate) struct Store(pub Session);

impl DiagnosticPort for Store {
    fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                Ok(graph_diagnostics::Entity::find()
                    .filter(graph_diagnostics::Column::RepositoryId.eq(id))
                    .order_by_asc(graph_diagnostics::Column::Ordinal)
                    .all(c)
                    .await?
                    .into_iter()
                    .map(diagnostic_value)
                    .collect())
            })
        })
    }
    fn paths(&self, id: &str, paths: &[String], limit: usize) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                let mut rows = Vec::new();
                for batch in paths.chunks(900) {
                    rows.extend(
                        graph_diagnostics::Entity::find()
                            .filter(graph_diagnostics::Column::RepositoryId.eq(id))
                            .filter(graph_diagnostics::Column::Path.is_in(batch.iter().cloned()))
                            .order_by_asc(graph_diagnostics::Column::Ordinal)
                            .limit(limit as u64)
                            .all(c)
                            .await?,
                    );
                }
                rows.sort_by_key(|d| d.ordinal);
                rows.dedup_by_key(|d| d.ordinal);
                Ok(rows.into_iter().take(limit).map(diagnostic_value).collect())
            })
        })
    }
    fn errors(&self, id: &str) -> Result<i64> {
        self.0.with(|c| {
            run(async {
                Ok(graph_diagnostics::Entity::find()
                    .filter(graph_diagnostics::Column::RepositoryId.eq(id))
                    .filter(graph_diagnostics::Column::Severity.eq("error"))
                    .count(c)
                    .await? as i64)
            })
        })
    }
    fn replace(&self, id: &str, artifact: &Value) -> Result<()> {
        self.0.with(|c| {
            run(async {
                graph_diagnostics::Entity::delete_many()
                    .filter(graph_diagnostics::Column::RepositoryId.eq(id))
                    .exec(c)
                    .await?;
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
                graph_inserts::insert::<graph_diagnostics::Entity>(c, rows).await
            })
        })
    }
}
