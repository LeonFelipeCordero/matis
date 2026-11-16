use crate::{
    domain::{
        navigation::{ProjectSelector, error},
        repositories::{RepositoryDto, RepositoryError},
        storage::RepositoryPort,
    },
    paths::absolute_path,
};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;
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
    repository: Box<dyn RepositoryPort>,
}
impl RepositoryService {
    pub fn new(repository: impl RepositoryPort + 'static) -> Self {
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
    pub(crate) fn identifier(&mut self, identifier: &str) -> Result<String> {
        let id = required_text(identifier, "repositoryId", 200)?;
        if !self.repository.exists(&id)? {
            return Err(RepositoryError::new("Repository not found", 404).into());
        }
        Ok(id)
    }

    pub(crate) fn resolve(&self, name: &str) -> Result<Option<String>> {
        self.repository.resolve(name)
    }
    #[cfg(test)]
    pub(crate) fn exists(&self, id: &str) -> Result<bool> {
        self.repository.exists(id)
    }
    pub(crate) fn register_location(&self, name: &str, location: &str) -> Result<RepositoryDto> {
        self.repository.register(name, location)
    }
    pub(crate) fn project_identifier(&self, name: &str) -> Result<String> {
        let name = required_text(name, "project name", 200)?;
        self.repository.resolve(&name)?.ok_or_else(|| RepositoryError::new("Repository not found", 404).into())
    }
    pub(crate) fn select(&self, project: ProjectSelector<'_>) -> Result<(String, String)> {
        let rows = match project {
            ProjectSelector::Name(name) => {
                self.repository.select(name)?.into_iter().map(|r| (r.id, r.location)).collect()
            }
            ProjectSelector::Directory(directory) => self.containing_projects(directory)?,
        };
        if rows.len() != 1 {
            return Err(error("Project not found or ambiguous; use an explicit registered project ID with --project"));
        }
        Ok(rows.into_iter().next().unwrap())
    }
    fn containing_projects(&self, directory: &Path) -> Result<Vec<(String, String)>> {
        let current = absolute_path(directory)?;
        let mut matches = Vec::new();
        for project in self.repository.list()? {
            let (id, location) = (project.id, project.location);
            if location.contains("://") {
                continue;
            }
            let root = absolute_path(Path::new(&location))?;
            if current.starts_with(&root) {
                matches.push((root.components().count(), id, location));
            }
        }
        if matches.is_empty() {
            return Err(error(
                "No indexed project contains the current directory; run matis index from the repository root or use --project",
            ));
        }
        let nearest = matches.iter().map(|m| m.0).max().unwrap();
        Ok(matches.into_iter().filter(|m| m.0 == nearest).map(|(_, id, location)| (id, location)).collect())
    }
}
