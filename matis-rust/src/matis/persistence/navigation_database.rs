use super::database::{connect, database_path, run};
use crate::domain::navigation::error;
use anyhow::Result;
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbBackend, Statement, TransactionTrait};
use std::{collections::BTreeSet, path::Path};

const REQUIRED_TABLES: &[&str] = &[
    "repositories",
    "repository_graphs",
    "graph_nodes",
    "graph_files",
    "graph_symbols",
    "graph_node_metrics",
    "graph_node_decorators",
    "graph_edges",
    "graph_diagnostics",
];
const REQUIRED_INDEXES: &[&str] = &[
    "graph_nodes_parent",
    "graph_nodes_name",
    "graph_nodes_path",
    "graph_edges_source",
    "graph_edges_target",
    "graph_diagnostics_path",
];
async fn check_schema(connection: &DatabaseTransaction) -> Result<()> {
    let names: BTreeSet<String> = connection
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type IN ('table','index')",
        ))
        .await?
        .into_iter()
        .map(|s| s.try_get("", "name"))
        .collect::<Result<_, _>>()?;
    let missing: BTreeSet<&str> =
        REQUIRED_TABLES.iter().chain(REQUIRED_INDEXES).copied().filter(|n| !names.contains(*n)).collect();
    if !missing.is_empty() {
        return Err(error(format!(
            "Missing graph schema/index: {}. Explicitly migrate and index the project.",
            missing.into_iter().collect::<Vec<_>>().join(", ")
        )));
    }
    Ok(())
}
pub struct ReadTransaction(Option<DatabaseTransaction>);
impl std::ops::Deref for ReadTransaction {
    type Target = DatabaseTransaction;
    /// Borrow the active read transaction; panic if its internal transaction is absent.
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("Read transaction")
    }
}
impl Drop for ReadTransaction {
    /// Drop the transaction inside Tokio so SQLx can schedule rollback and pool cleanup.
    fn drop(&mut self) {
        // SQLx needs Tokio to schedule rollback and return the connection to its pool.
        run(async {
            drop(self.0.take());
        });
    }
}
/// Open the supplied/default database in read-only mode and retain one read transaction.
/// Check required graph tables/indexes; never create or migrate a database. Dropping
/// the returned guard schedules rollback and releases its connection.
///
/// # Errors
/// Return path, connection, query, or missing-schema errors with migration/indexing guidance.
pub fn readonly_session(path: Option<&Path>) -> Result<ReadTransaction> {
    let target = database_path(path)?;
    run(async {
        let db = connect(&target, true).await.map_err(|e| {
            let message = e.to_string();
            let detail = if message.contains("unable to open database") {
                "(sqlite3.OperationalError) unable to open database file\n(Background on this error at: https://sqlalche.me/e/21/e3q8)".to_owned()
            } else { message };
            error(format!("Cannot read graph database {}: {detail}. Index the project after explicitly migrating the database.", target.display()))
        })?;
        db.execute_unprepared("PRAGMA query_only=ON").await?;
        let tx = db.begin().await?;
        check_schema(&tx).await?;
        Ok(ReadTransaction(Some(tx)))
    })
}
