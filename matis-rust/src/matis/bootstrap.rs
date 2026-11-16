pub use crate::persistence::database::migrate_database;
use crate::{
    domain::navigation::ProjectSelector,
    persistence::{self, session::Session},
    services::{
        diagnostics::DiagnosticService,
        graph_revisions::GraphRevisionService,
        mutation_candidates::MutationCandidateService,
        navigation::NavigationService,
        navigation_reader::CapabilityReader,
        node_metrics::NodeMetricsService,
        nodes::{NodeRepositories, NodeService},
        relationships::RelationshipService,
        repositories::RepositoryService,
        repository_workflow::RepositoryWorkflow,
        saved_graph::SavedGraphWorkflow,
    },
};
use anyhow::Result;
use std::path::Path;

fn repository(session: &Session) -> RepositoryService {
    RepositoryService::new(persistence::repositories::Store(session.clone()))
}
fn graph(session: &Session) -> SavedGraphWorkflow {
    SavedGraphWorkflow {
        revisions: GraphRevisionService::new(persistence::revisions::Store(session.clone())),
        nodes: NodeService::new(NodeRepositories {
            nodes: Box::new(persistence::nodes::Store(session.clone())),
            files: Box::new(persistence::files::Store(session.clone())),
            symbols: Box::new(persistence::symbols::Store(session.clone())),
            decorators: Box::new(persistence::decorators::Store(session.clone())),
        }),
        relationships: RelationshipService::new(persistence::relationships::Store(session.clone())),
        metrics: NodeMetricsService::new(persistence::metrics::Store(session.clone())),
        diagnostics: DiagnosticService::new(persistence::diagnostics::Store(session.clone())),
        candidates: MutationCandidateService::new(persistence::candidates::Store(session.clone())),
    }
}
/// Open a workflow without migrating. All capabilities share its transaction scope.
pub fn repository_service(path: Option<&Path>) -> Result<RepositoryWorkflow> {
    let session = Session::writable(path)?;
    Ok(RepositoryWorkflow::new(repository(&session), graph(&session), session))
}
/// Compose capabilities on one read-only snapshot. Never create or migrate storage.
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
pub(crate) fn navigation_reader(
    project: ProjectSelector<'_>,
    revision: Option<&str>,
    path: Option<&Path>,
) -> Result<CapabilityReader> {
    let session = Session::readonly(path)?;
    CapabilityReader::new(repository(&session), graph(&session), project)?.at_revision(revision)
}
