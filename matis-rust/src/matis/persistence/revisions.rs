use super::{
    database::run,
    models::{RepositoryGraphRow, repository_graphs},
    session::Session,
};
#[path = "graph_metadata.rs"]
mod metadata;
use super::identity::{now, uuid};
use crate::domain::repositories::{GraphDto, GraphStatsDto};
use crate::domain::storage::GraphRevisionPort;
use anyhow::Result;
use metadata::GraphMetadata;
use sea_orm::{ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, sea_query::OnConflict};
use serde_json::Value;
use std::collections::HashMap;
pub(crate) struct Store(pub Session);

fn dto(row: RepositoryGraphRow) -> GraphDto {
    GraphDto {
        repository_id: row.repository_id,
        graph_revision: row.graph_revision.unwrap_or_default(),
        schema_version: row.schema_version,
        indexed_at: row.indexed_at,
        root_name: row.root_name,
        root_path: row.root_path,
        imported_at: row.imported_at,
        stats: GraphStatsDto::default(),
    }
}
impl GraphRevisionPort for Store {
    fn get(&self, id: &str) -> Result<Option<GraphDto>> {
        self.0.with(|c| {
            run(async {
                Ok(repository_graphs::Entity::find_by_id(id)
                    .one(c)
                    .await?
                    .filter(|g| g.graph_revision.is_some())
                    .map(dto))
            })
        })
    }
    fn imports(&self) -> Result<HashMap<String, i64>> {
        self.0.with(|c| {
            run(async {
                Ok(repository_graphs::Entity::find()
                    .filter(repository_graphs::Column::GraphRevision.is_not_null())
                    .all(c)
                    .await?
                    .into_iter()
                    .map(|g| (g.repository_id, g.imported_at))
                    .collect())
            })
        })
    }
    fn save(&self, id: &str, artifact: &Value) -> Result<GraphDto> {
        self.0.with(|c| {
            run(async {
                let metadata = GraphMetadata::read(c, artifact).await?;
                let row = RepositoryGraphRow {
                    repository_id: id.into(),
                    imported_at: now(),
                    graph_revision: Some(uuid(true)?),
                    schema_version: metadata.schema_version,
                    indexed_at: metadata.indexed_at,
                    root_name: metadata.root_name,
                    root_path: metadata.root_path,
                };
                use repository_graphs::Column as G;
                repository_graphs::Entity::insert(row.clone().into_active_model())
                    .on_conflict(
                        OnConflict::column(G::RepositoryId)
                            .update_columns([
                                G::ImportedAt,
                                G::GraphRevision,
                                G::SchemaVersion,
                                G::IndexedAt,
                                G::RootName,
                                G::RootPath,
                            ])
                            .to_owned(),
                    )
                    .exec(c)
                    .await?;
                Ok(dto(row))
            })
        })
    }
}
