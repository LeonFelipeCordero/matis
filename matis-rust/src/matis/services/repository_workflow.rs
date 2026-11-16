use super::{repositories::RepositoryService, saved_graph::SavedGraphWorkflow};
pub use crate::indexer::analyze_repository as analyze_folder;
use crate::{
    domain::{
        graph_artifact::validate_parentage,
        repositories::{GraphDto, RepositoryDto, RepositoryError},
        storage::UnitOfWork,
    },
    paths::absolute_path,
};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;
pub struct RepositoryWorkflow {
    repository: RepositoryService,
    graph: SavedGraphWorkflow,
    unit: Box<dyn UnitOfWork>,
}
impl RepositoryWorkflow {
    pub fn new(repository: RepositoryService, graph: SavedGraphWorkflow, unit: impl UnitOfWork + 'static) -> Self {
        Self { repository, graph, unit: Box::new(unit) }
    }
    fn transaction<T>(&mut self, operation: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let scope = self.unit.begin()?;
        let result = operation(self)?;
        scope.commit()?;
        Ok(result)
    }
    pub fn register(&mut self, input: &Value) -> Result<RepositoryDto> {
        self.transaction(|s| s.repository.register(input))
    }
    pub fn list(&mut self) -> Result<Vec<RepositoryDto>> {
        self.transaction(|s| {
            let imports = s.graph.revisions.imports().map_err(storage_error)?;
            Ok(s.repository
                .list()?
                .into_iter()
                .map(|mut r| {
                    r.imported_at = imports.get(&r.id).copied();
                    r
                })
                .collect())
        })
    }
    pub fn get_architecture(&mut self, identifier: &str) -> Result<Option<GraphDto>> {
        self.transaction(|s| {
            let id = s.repository.identifier(identifier)?;
            s.graph.summary(&id).map_err(storage_error)
        })
    }
    pub fn get_report_facts(&mut self, identifier: &str) -> Result<Option<Value>> {
        self.transaction(|s| {
            let id = s.repository.identifier(identifier)?;
            s.graph.report_facts(&id).map_err(storage_error)
        })
    }
    pub fn report(&mut self, name: &str) -> Result<Value> {
        self.transaction(|s| s.report_inner(name))
    }
    fn report_inner(&mut self, name: &str) -> Result<Value> {
        let id = self.repository.project_identifier(name)?;
        let facts = self
            .graph
            .report_facts(&id)
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::new("Repository has no indexed graph; run matis index first", 409))?;
        crate::metrics::report::build_report(&facts)
    }
    pub fn saved_folder_report(&mut self, root: &Path) -> Result<Value> {
        self.transaction(|s| s.saved_folder_report_inner(root))
    }
    pub fn saved_folder_candidates(&mut self, root: &Path) -> Result<Vec<Value>> {
        self.transaction(|s| s.saved_folder_candidates_inner(root))
    }
    pub fn index(&mut self, name: &str, root: &Path) -> Result<GraphDto> {
        // Analyze before opening a write transaction.
        let id = self.transaction(|s| s.repository.project_identifier(name))?;
        let artifact = analyze_folder(root)?;
        self.save(&id, &artifact)
    }
    pub(crate) fn resolve(&mut self, name: &str) -> Result<Option<String>> {
        self.transaction(|s| s.repository.resolve(name))
    }
    #[cfg(test)]
    pub(crate) fn exists(&mut self, id: &str) -> Result<bool> {
        self.transaction(|s| s.repository.exists(id))
    }
    pub(crate) fn save(&mut self, id: &str, artifact: &Value) -> Result<GraphDto> {
        validate_parentage(artifact)?;
        self.transaction(|s| s.graph.save(id, artifact).map_err(storage_error))
    }
    pub(crate) fn save_new(&mut self, name: &str, location: &str, artifact: &Value) -> Result<GraphDto> {
        validate_parentage(artifact)?;
        self.transaction(|s| s.save_new_inner(name, location, artifact).map_err(storage_error))
    }
    fn save_new_inner(&mut self, name: &str, location: &str, artifact: &Value) -> Result<GraphDto> {
        let registered = self.repository.register_location(name, location)?;
        self.graph.save(&registered.id, artifact)
    }
    fn folder_project_name(&mut self, root: &Path) -> Result<String> {
        let root = absolute_path(root)?;
        let projects = self
            .repository
            .list()?
            .into_iter()
            .filter(|p| !p.location.contains("://"))
            .map(|p| Ok((absolute_path(Path::new(&p.location))?, p)))
            .collect::<Result<Vec<_>>>()?;
        let mut matching: Vec<_> = projects.into_iter().filter(|(location, _)| *location == root).collect();
        anyhow::ensure!(matching.len() == 1, "Expected one indexed project at this folder; run matis index first");
        Ok(matching.remove(0).1.name)
    }
    /// Build a report for the sole registration whose resolved local location equals `root`.
    /// Ignore URLs; propagate path, storage, name, ambiguity, and report errors. Do not index.
    fn saved_folder_report_inner(&mut self, root: &Path) -> Result<Value> {
        let name = self.folder_project_name(root)?;
        self.report_inner(&name)
    }
    /// Read ordered saved candidates for the sole local registration at `root`, without reading sources.
    /// Use the same selection/name errors as `saved_folder_report`; return status 409 without a graph.
    fn saved_folder_candidates_inner(&mut self, root: &Path) -> Result<Vec<Value>> {
        let name = self.folder_project_name(root)?;
        let id = self.repository.project_identifier(&name)?;
        self.graph
            .mutation_candidates(&id)
            .map_err(storage_error)?
            .ok_or_else(|| RepositoryError::new("Repository has no indexed graph; run matis index first", 409).into())
    }
    /// Save `artifact` by resolved local `root`, returning its graph summary without analyzing sources.
    /// Update the first matching local registration, ignoring URLs; otherwise choose an unused
    /// folder-based name and register/save atomically. Propagate path/store errors;
    /// failed store replacements preserve the previous graph.
    pub fn save_folder(&mut self, root: &Path, artifact: &Value) -> Result<GraphDto> {
        let root = absolute_path(root)?;
        for project in self.list()? {
            if !project.location.contains("://") && absolute_path(Path::new(&project.location))? == root {
                return self.save(&project.id, artifact);
            }
        }
        let base = root.file_name().unwrap_or_default().to_string_lossy().trim().chars().take(190).collect::<String>();
        let base = if base.is_empty() { "repository".into() } else { base };
        let mut name = base.clone();
        let mut suffix = 2;
        while self.resolve(&name)?.is_some() {
            name = format!("{base} ({suffix})");
            suffix += 1;
        }
        self.save_new(&name, &root.to_string_lossy(), artifact)
    }
}
fn storage_error(error: impl std::fmt::Display) -> anyhow::Error {
    RepositoryError::new(error.to_string(), 500).into()
}
