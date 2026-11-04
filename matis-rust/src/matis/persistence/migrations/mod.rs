use anyhow::{Context, Result, ensure};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, Set, Statement, TransactionTrait,
};
use sea_orm_migration::{prelude::*, seaql_migrations};

pub const REVISIONS: &[(&str, &str)] = &[
    ("0001_initial", include_str!("versions/0001_initial.sql")),
    ("0002_graph_nodes", include_str!("versions/0002_graph_nodes.sql")),
    ("0003_graph_relations", include_str!("versions/0003_graph_relations.sql")),
    ("0004_drop_graph_artifact", include_str!("versions/0004_drop_graph_artifact.sql")),
    ("0005_mutation_candidates", include_str!("versions/0005_mutation_candidates.sql")),
];
pub struct Migrator;
#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// Return the ordered SQL revisions shared with Python, without applying them.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        REVISIONS
            .iter()
            .map(|&(revision, sql)| Box::new(Revision { revision, sql }) as Box<dyn MigrationTrait>)
            .collect()
    }
}
struct Revision {
    revision: &'static str,
    sql: &'static str,
}
impl MigrationName for Revision {
    /// Return the revision identifier used in migration history.
    fn name(&self) -> &str {
        self.revision
    }
}
#[async_trait::async_trait]
impl MigrationTrait for Revision {
    /// Execute this revision's SQL and update/initialize the Alembic version row.
    /// Propagate database failures; transaction control belongs to the migration caller.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        db.execute_unprepared(self.sql).await?;
        db.execute(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "UPDATE alembic_version SET version_num = ?",
            [self.revision.into()],
        ))
        .await?;
        // A fresh database has no Alembic row yet.
        db.execute(Statement::from_sql_and_values(
            DbBackend::Sqlite,
            "INSERT INTO alembic_version(version_num) SELECT ? WHERE NOT EXISTS (SELECT 1 FROM alembic_version)",
            [self.revision.into()],
        ))
        .await?;
        Ok(())
    }
    /// Reject every downgrade with a database error; use Python/Alembic instead.
    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom("Schema downgrades are not supported; restore a database backup instead".into()))
    }
}

/// Enable WAL and atomically adopt compatible Alembic history and apply pending SQL revisions.
///
/// # Errors
/// Reject unknown, multiple, or inconsistent revision histories. Roll back failed
/// schema transactions; retry SQLite busy errors for up to 30 seconds, then fail.
/// WAL mode is set outside the schema transaction and is not rolled back.
pub async fn migrate(db: &DatabaseConnection) -> Result<()> {
    migrate_with_timeout(db, std::time::Duration::from_secs(30)).await
}
pub(super) async fn migrate_with_timeout(db: &DatabaseConnection, timeout: std::time::Duration) -> Result<()> {
    // SeaORM 1.1 cannot request BEGIN IMMEDIATE; retry with a new snapshot after rollback.
    tokio::time::timeout(timeout, async {
        loop {
            match migrate_once(db).await {
                Err(error) if sqlite_busy(&error) => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                result => return result,
            }
        }
    })
    .await
    .with_context(|| {
        format!("Timed out waiting for SQLite migration write lock after {} seconds", timeout.as_secs_f64())
    })?
}
fn sqlite_busy(error: &anyhow::Error) -> bool {
    error.chain().filter_map(|cause| cause.downcast_ref::<sea_orm::SqlxError>()).any(|error| {
        error
            .as_database_error()
            .and_then(|error| error.code())
            .and_then(|code| code.parse::<i32>().ok())
            .is_some_and(|code| code & 0xff == 5)
    })
}
async fn migrate_once(db: &DatabaseConnection) -> Result<()> {
    db.execute_unprepared("PRAGMA journal_mode=WAL;").await?;
    let tx = db.begin().await?;
    // Serialize version adoption and schema changes in the same transaction.
    tx.execute_unprepared(&include_str!("alembic_version.sql").replacen(
        "CREATE TABLE",
        "CREATE TABLE IF NOT EXISTS",
        1,
    ))
    .await?;
    tx.execute_unprepared("UPDATE alembic_version SET version_num=version_num").await?;
    let rows =
        tx.query_all(Statement::from_string(DbBackend::Sqlite, "SELECT version_num FROM alembic_version")).await?;
    ensure!(rows.len() <= 1, "Multiple Alembic revisions are not supported");
    let revision = rows.first().map(|r| r.try_get::<String>("", "version_num")).transpose()?;
    let adopted = match revision {
        None => 0,
        Some(ref revision) => {
            REVISIONS
                .iter()
                .position(|(id, _)| id == revision)
                .with_context(|| format!("Unknown Alembic revision: {revision}"))?
                + 1
        }
    };
    Migrator::install(&tx).await?;
    let applied = seaql_migrations::Entity::find().all(&tx).await?;
    for row in &applied {
        let index = REVISIONS
            .iter()
            .position(|(id, _)| *id == row.version)
            .with_context(|| format!("Unknown SeaORM migration: {}", row.version))?;
        ensure!(index < adopted, "SeaORM migration history is ahead of Alembic; refusing schema replay");
    }
    for &(revision, _) in &REVISIONS[..adopted] {
        if !applied.iter().any(|r| r.version == revision) {
            seaql_migrations::ActiveModel {
                version: Set(revision.into()),
                applied_at: Set(chrono::Utc::now().timestamp()),
            }
            .insert(&tx)
            .await?;
        }
    }
    Migrator::up(&tx, None).await?;
    tx.commit().await?;
    Ok(())
}
