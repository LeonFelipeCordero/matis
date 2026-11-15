use crate::{
    domain::repositories::{GraphDto, RepositoryDto, RepositoryError, RepositoryProtocol},
    paths::absolute_path,
};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub use crate::indexer::analyze_repository as analyze_folder;
fn required_text(value: &str, field: &str, maximum: usize) -> Result<String> {
    let text = value.trim();
    if text.is_empty() {
        return Err(RepositoryError::new(format!("{field} is required"), 400).into());
    }
    if text.chars().count() > maximum {
        return Err(RepositoryError::new(format!("{field} must be at most {maximum} characters"), 400).into());
    }
    Ok(text.into())
}
pub struct RepositoryService {
    repository: Box<dyn RepositoryProtocol>,
}
impl RepositoryService {
    /// Own a repository implementation without opening storage or performing operations on it.
    pub fn new(repository: impl RepositoryProtocol + 'static) -> Self {
        Self { repository: Box::new(repository) }
    }
    /// Validate/trim input name and location, persist a registration, and return its DTO.
    /// Require an object with nonempty strings of at most 200/4096 characters; return
    /// status 400 for invalid input and propagate store errors. Do not index the location.
    pub fn register(&mut self, input: &Value) -> Result<RepositoryDto> {
        if !input.is_object() {
            return Err(RepositoryError::new("Expected an object with name and location", 400).into());
        }
        self.repository.register(
            &required_text(input["name"].as_str().unwrap_or(""), "name", 200)?,
            &required_text(input["location"].as_str().unwrap_or(""), "location", 4096)?,
        )
    }
    /// Return registrations and their latest graph import times; propagate store errors.
    pub fn list(&mut self) -> Result<Vec<RepositoryDto>> {
        self.repository.list()
    }
    fn identifier(&mut self, identifier: &str) -> Result<String> {
        let id = required_text(identifier, "repositoryId", 200)?;
        if !self.repository.exists(&id)? {
            return Err(RepositoryError::new("Repository not found", 404).into());
        }
        Ok(id)
    }
    /// Return the ID's graph summary, or `None` when registered but not indexed.
    /// Trim/validate the ID (200-character maximum); return status 400 for invalid IDs,
    /// 404 for missing registrations, or propagate store errors.
    pub fn get_architecture(&mut self, identifier: &str) -> Result<Option<GraphDto>> {
        let id = self.identifier(identifier)?;
        self.repository.graph(&id)
    }
    /// Return the ID's saved report inputs, or `None` when not indexed.
    /// Trim/validate the ID (200-character maximum); return status 400 for invalid IDs,
    /// 404 for missing registrations, or propagate store errors.
    pub fn get_report_facts(&mut self, identifier: &str) -> Result<Option<Value>> {
        let id = self.identifier(identifier)?;
        self.repository.report_facts(&id)
    }
    fn project_identifier(&mut self, project_name: &str) -> Result<String> {
        let name = required_text(project_name, "project name", 200)?;
        self.repository.resolve(&name)?.ok_or_else(|| RepositoryError::new("Repository not found", 404).into())
    }
    /// Build a report from saved facts for a case-insensitive project name without reading source files.
    /// Return status 400 for invalid/overlong names, 404 for missing registrations,
    /// 409 for unindexed projects, or propagate store/report errors. Do not save changes.
    pub fn report(&mut self, project_name: &str) -> Result<Value> {
        let id = self.project_identifier(project_name)?;
        let facts = self
            .repository
            .report_facts(&id)?
            .ok_or_else(|| RepositoryError::new("Repository has no indexed graph; run matis index first", 409))?;
        crate::metrics::report::build_report(&facts)
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
    pub fn saved_folder_report(&mut self, root: &Path) -> Result<Value> {
        let name = self.folder_project_name(root)?;
        self.report(&name)
    }
    /// Read ordered saved candidates for the sole local registration at `root`, without reading sources.
    /// Use the same selection/name errors as `saved_folder_report`; return status 409 without a graph.
    pub fn saved_folder_candidates(&mut self, root: &Path) -> Result<Vec<Value>> {
        let name = self.folder_project_name(root)?;
        let id = self.project_identifier(&name)?;
        self.repository
            .mutation_candidates(&id)?
            .ok_or_else(|| RepositoryError::new("Repository has no indexed graph; run matis index first", 409).into())
    }
    /// Save `artifact` by resolved local `root`, returning its graph summary without analyzing sources.
    /// Update the first matching local registration, ignoring URLs; otherwise choose an unused
    /// folder-based name and register/save atomically. Propagate path/store errors;
    /// failed store replacements preserve the previous graph.
    pub fn save_folder(&mut self, root: &Path, artifact: &Value) -> Result<GraphDto> {
        let root = absolute_path(root)?;
        for project in self.repository.list()? {
            if !project.location.contains("://") && absolute_path(Path::new(&project.location))? == root {
                return self.repository.save(&project.id, artifact);
            }
        }
        let base = root.file_name().unwrap_or_default().to_string_lossy().trim().chars().take(190).collect::<String>();
        let base = if base.is_empty() { "repository".into() } else { base };
        let mut name = base.clone();
        let mut suffix = 2;
        while self.repository.resolve(&name)?.is_some() {
            name = format!("{base} ({suffix})");
            suffix += 1;
        }
        self.repository.save_new(&name, &root.to_string_lossy(), artifact)
    }
    /// Analyze `root`, then atomically replace the case-insensitive named project's graph.
    /// Return its summary; resolve by name, not root location. Invalid names produce status
    /// 400 and missing projects 404. Analysis errors occur before saving; store errors propagate.
    pub fn index(&mut self, project_name: &str, root: &Path) -> Result<GraphDto> {
        let id = self.project_identifier(project_name)?;
        let artifact = analyze_folder(root)?;
        self.repository.save(&id, &artifact)
    }
}
