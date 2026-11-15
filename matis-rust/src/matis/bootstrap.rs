use crate::persistence::{
    navigation::{ProjectSelector, navigation_reader},
    repositories::repository_store,
};
use crate::services::{navigation::NavigationService, repositories::RepositoryService};
use anyhow::Result;
use std::path::Path;

pub use crate::persistence::database::migrate_database;
/// Open a repository service at the supplied/default database path without migrating.
/// May create the file/directories; propagate open/path errors. Store methods own write commits.
pub fn repository_service(path: Option<&Path>) -> Result<RepositoryService> {
    Ok(RepositoryService::new(repository_store(path)?))
}
/// Open navigation on one read-only snapshot at the supplied/default database path.
/// Select `project` by ID/name, or the nearest project containing cwd when omitted;
/// require `revision` to match when supplied. Dropping the service releases the snapshot.
/// Return cwd/path, schema, missing/ambiguous project, stale revision, or database errors.
/// Never create or migrate storage.
pub fn navigation_service(
    project: Option<&str>,
    revision: Option<&str>,
    path: Option<&Path>,
) -> Result<NavigationService> {
    let cwd = std::env::current_dir()?;
    let selector = match project {
        Some(name) => ProjectSelector::Name(name),
        None => ProjectSelector::Directory(&cwd),
    };
    Ok(NavigationService::new(navigation_reader(selector, revision, path)?))
}
