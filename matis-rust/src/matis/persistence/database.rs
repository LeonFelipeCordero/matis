use crate::domain::repositories::RepositoryError;
use anyhow::Result;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::{
    env,
    future::Future,
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

pub use crate::paths::{absolute_path, expand_home};

// Keep the async pool alive across synchronous CLI/repository calls.
/// Block on `future` using the process-wide Tokio runtime and return its output.
///
/// # Panics
/// Panic if runtime creation fails or the caller is already inside a Tokio runtime.
pub(crate) fn run<F: Future>(future: F) -> F::Output {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Tokio runtime")).block_on(future)
}
/// Return nonempty `DATABASE_PATH`, or `matis/matis.sqlite` under the user data directory.
/// Use nonempty `XDG_DATA_HOME` or `~/.local/share`; expand home markers without
/// creating files. Return an error if a home directory cannot be resolved.
pub fn default_database_path() -> Result<PathBuf> {
    if let Some(path) = env::var_os("DATABASE_PATH").filter(|s| !s.is_empty()) {
        return expand_home(Path::new(&path));
    }
    let data = env::var_os("XDG_DATA_HOME")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("~/.local/share"));
    Ok(expand_home(&data)?.join("matis/matis.sqlite"))
}
/// Resolve `path` or the default database path to an absolute, home-expanded path.
/// Resolve existing symlinks without requiring the file to exist; propagate path errors.
pub fn database_path(path: Option<&Path>) -> Result<PathBuf> {
    absolute_path(&match path {
        Some(path) => path.to_path_buf(),
        None => default_database_path()?,
    })
}
/// Open a single-connection SQLite pool at `target`, enabling foreign keys and a 30-second busy timeout.
/// `readonly` forbids file creation; otherwise SQLite may create the file.
/// Return connection/configuration failures as repository errors with status 500; do not migrate.
pub(crate) async fn connect(target: &Path, readonly: bool) -> Result<DatabaseConnection> {
    let encoded = target.to_string_lossy().replace('%', "%25").replace('?', "%3F").replace('#', "%23");
    let mut options = ConnectOptions::new(format!("sqlite://{encoded}?mode={}", if readonly { "ro" } else { "rwc" }));
    options.max_connections(1).min_connections(1).sqlx_logging(false).connect_timeout(Duration::from_secs(30));
    let db = Database::connect(options).await.map_err(storage_error)?;
    use sea_orm::ConnectionTrait;
    db.execute_unprepared("PRAGMA busy_timeout=30000; PRAGMA foreign_keys=ON;").await.map_err(storage_error)?;
    Ok(db)
}
/// Open the supplied/default database, creating parent directories and the file if needed.
/// Enable foreign keys without migrating. Return path, filesystem, or connection errors.
pub fn create_engine(path: Option<&Path>) -> Result<DatabaseConnection> {
    let target = database_path(path)?;
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    run(connect(&target, false))
}
/// Convert a displayable storage failure to a repository error with status 500.
pub fn storage_error(error: impl std::fmt::Display) -> anyhow::Error {
    RepositoryError::new(error.to_string(), 500).into()
}
/// Create/open the supplied/default database, enable WAL, and apply pending migrations.
/// Commit schema changes together; roll back schema failures and wrap migration errors
/// as repository errors with status 500. Path and directory errors propagate directly.
pub fn migrate_database(path: Option<&Path>) -> Result<()> {
    let connection = create_engine(path)?;
    run(super::migrations::migrate(&connection)).map_err(storage_error)
}
/// Open a writable connection at the supplied/default path without migrating or starting a transaction.
/// May create the file and parent directories; propagate path, filesystem, and connection errors.
pub fn repository_session(path: Option<&Path>) -> Result<DatabaseConnection> {
    create_engine(path)
}
