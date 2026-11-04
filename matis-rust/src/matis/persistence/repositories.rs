use super::{
    database::{create_engine, run, storage_error},
    graph_queries,
    graph_records::*,
    models::*,
};
use crate::domain::repositories::{GraphDto, RepositoryDto, RepositoryError, RepositoryProtocol};
use anyhow::Result;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    TransactionTrait,
    sea_query::{Expr, Func, OnConflict},
};
use serde_json::Value;
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct RepositoryStore {
    pub(crate) connection: DatabaseConnection,
}
/// Open a store at the supplied/default database path without migrating.
/// May create the database and parent directories; propagate path/open errors.
pub fn repository_store(path: Option<&Path>) -> Result<RepositoryStore> {
    Ok(RepositoryStore { connection: create_engine(path)? })
}
fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}
fn uuid(compact: bool) -> Result<String> {
    let mut data = [0u8; 16];
    getrandom::fill(&mut data)?;
    data[6] = (data[6] & 15) | 64;
    data[8] = (data[8] & 63) | 128;
    let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
    Ok(if compact {
        hex
    } else {
        format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
    })
}
fn dto(r: RepositoryRow, imported_at: Option<i64>) -> RepositoryDto {
    RepositoryDto { id: r.id, name: r.name, location: r.location, created_at: r.created_at, imported_at }
}
fn registration(name: &str, location: &str) -> Result<RepositoryRow> {
    Ok(RepositoryRow { id: uuid(false)?, name: name.into(), location: location.into(), created_at: now() })
}
fn duplicate_error(error: sea_orm::DbErr) -> anyhow::Error {
    let message = error.to_string();
    if message.contains("repositories_name_unique") || message.contains("repositories.name") {
        return RepositoryError::new("A repository with this name already exists", 409).into();
    }
    storage_error(error)
}
/// Build a literal SQLite case-insensitive repository-name comparison without querying.
pub(crate) fn name_matches(name: &str) -> sea_orm::sea_query::SimpleExpr {
    Expr::expr(Func::lower(Expr::col(repositories::Column::Name))).eq(Func::lower(Expr::val(name)))
}
impl RepositoryProtocol for RepositoryStore {
    /// Insert and commit a name/location registration without indexing; return its DTO.
    /// Report duplicate names as status 409, other database errors as 500, and propagate RNG errors.
    fn register(&mut self, name: &str, location: &str) -> Result<RepositoryDto> {
        let row = registration(name, location)?;
        run(repositories::Entity::insert(row.clone().into_active_model()).exec(&self.connection))
            .map_err(duplicate_error)?;
        Ok(dto(row, None))
    }
    /// Read registrations by creation time/ID with revisioned graph import times.
    /// Return database failures as repository errors with status 500.
    fn list(&mut self) -> Result<Vec<RepositoryDto>> {
        let rows = run(repositories::Entity::find()
            .find_also_related(repository_graphs::Entity)
            .order_by_asc(repositories::Column::CreatedAt)
            .order_by_asc(repositories::Column::Id)
            .all(&self.connection))
        .map_err(storage_error)?;
        Ok(rows
            .into_iter()
            .map(|(r, g)| dto(r, g.filter(|g| g.graph_revision.is_some()).map(|g| g.imported_at)))
            .collect())
    }
    /// Check the registration for repository ID `identifier`; wrap database failures as status 500.
    fn exists(&mut self, identifier: &str) -> Result<bool> {
        Ok(run(repositories::Entity::find_by_id(identifier).one(&self.connection)).map_err(storage_error)?.is_some())
    }
    /// Read an ID by literal case-insensitive `name`, or `None` if absent.
    /// Wrap database failures as repository errors with status 500.
    fn resolve(&mut self, name: &str) -> Result<Option<String>> {
        Ok(run(repositories::Entity::find().filter(name_matches(name)).one(&self.connection))
            .map_err(storage_error)?
            .map(|r| r.id))
    }
    /// Read the ID's summary in one transaction; return `None` without a revisioned graph.
    /// Wrap query/transaction failures as repository errors with status 500; do not modify graph rows.
    fn graph(&mut self, identifier: &str) -> Result<Option<GraphDto>> {
        run(async {
            let tx = self.connection.begin().await?;
            let result = match graph_queries::graph_row(&tx, identifier).await? {
                Some(row) => Some(graph_queries::graph_summary(&tx, row).await?),
                None => None,
            };
            tx.commit().await?;
            Ok::<_, anyhow::Error>(result)
        })
        .map_err(storage_error)
    }
    /// Read one revision's report inputs in one transaction, or `None` without a saved graph.
    /// Wrap query, JSON, and transaction failures as status 500; do not read source files.
    fn report_facts(&mut self, identifier: &str) -> Result<Option<Value>> {
        run(async {
            let tx = self.connection.begin().await?;
            let result = match graph_queries::graph_row(&tx, identifier).await? {
                Some(row) => Some(graph_queries::report_facts(&tx, row).await?),
                None => None,
            };
            tx.commit().await?;
            Ok::<_, anyhow::Error>(result)
        })
        .map_err(storage_error)
    }
    /// Validate `artifact`, replace the ID's graph atomically, and return a new revision summary.
    /// Shape/cycle errors propagate before writes. Wrap transactional failures as status 500;
    /// rollback preserves the old graph, including on metadata or foreign-key failures.
    fn save(&mut self, identifier: &str, artifact: &Value) -> Result<GraphDto> {
        validate_parentage(artifact)?;
        run(async {
            let tx = self.connection.begin().await?;
            let result = save_graph(&tx, identifier, artifact).await?;
            tx.commit().await?;
            Ok::<_, anyhow::Error>(result)
        })
        .map_err(storage_error)
    }
    /// Validate `artifact`, then atomically register name/location and save its graph.
    /// Return a new revision summary. Propagate validation/RNG errors; wrap transactional
    /// failures as status 500 and roll back both registration and graph.
    fn save_new(&mut self, name: &str, location: &str, artifact: &Value) -> Result<GraphDto> {
        validate_parentage(artifact)?;
        let row = registration(name, location)?;
        run(async {
            let tx = self.connection.begin().await?;
            repositories::Entity::insert(row.clone().into_active_model()).exec(&tx).await?;
            let result = save_graph(&tx, &row.id, artifact).await?;
            tx.commit().await?;
            Ok::<_, anyhow::Error>(result)
        })
        .map_err(storage_error)
    }
}
async fn save_graph(connection: &impl ConnectionTrait, id: &str, artifact: &Value) -> Result<GraphDto> {
    let metadata = super::graph_metadata::GraphMetadata::read(connection, artifact).await?;
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
        .exec(connection)
        .await?;
    replace_graph(connection, id, artifact).await?;
    graph_queries::graph_summary(connection, row).await
}
async fn replace_graph(connection: &impl ConnectionTrait, id: &str, artifact: &Value) -> Result<()> {
    graph_edges::Entity::delete_many().filter(graph_edges::Column::RepositoryId.eq(id)).exec(connection).await?;
    graph_nodes::Entity::delete_many().filter(graph_nodes::Column::RepositoryId.eq(id)).exec(connection).await?;
    graph_diagnostics::Entity::delete_many()
        .filter(graph_diagnostics::Column::RepositoryId.eq(id))
        .exec(connection)
        .await?;
    let nodes = collection(artifact, "nodes").iter().enumerate().map(|(ordinal, n)| node_record(id, ordinal, n));
    super::graph_inserts::insert::<graph_nodes::Entity>(connection, nodes).await?;
    insert_node_details(connection, id, artifact).await?;
    insert_candidates(connection, id, artifact).await?;
    insert_edges(connection, id, artifact).await?;
    insert_diagnostics(connection, id, artifact).await
}
