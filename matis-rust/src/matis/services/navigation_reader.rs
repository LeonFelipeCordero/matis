use super::{repositories::RepositoryService, saved_graph::SavedGraphWorkflow};
use crate::domain::navigation::{
    ChildrenRequest, EdgeRequest, GraphSnapshot, NavigationReader, ProjectSelector, error,
};
use anyhow::Result;
use serde_json::{Value, json};
pub struct CapabilityReader {
    graph: SavedGraphWorkflow,
    snapshot: GraphSnapshot,
}
impl CapabilityReader {
    pub fn new(repository: RepositoryService, graph: SavedGraphWorkflow, project: ProjectSelector<'_>) -> Result<Self> {
        let (id, location) = repository.select(project)?;
        let saved =
            graph.revisions.get(&id)?.ok_or_else(|| error("Project has no normalized graph; index it first"))?;
        let snapshot = GraphSnapshot {
            repository_id: id,
            revision: saved.graph_revision,
            indexed_at: saved.indexed_at,
            root: location,
        };
        Ok(Self { graph, snapshot })
    }
    pub fn at_revision(self, revision: Option<&str>) -> Result<Self> {
        if let Some(expected) = revision.filter(|r| *r != self.snapshot.revision) {
            return Err(error(format!(
                "Stale graph revision: expected {expected}, current {}",
                self.snapshot.revision
            )));
        }
        Ok(self)
    }
}
impl NavigationReader for CapabilityReader {
    fn snapshot(&self) -> &GraphSnapshot {
        &self.snapshot
    }
    fn overview(&mut self) -> Result<Value> {
        let id = &self.snapshot.repository_id;
        let levels: Vec<_> = self
            .graph
            .nodes
            .levels(id)?
            .into_iter()
            .map(|(level, count)| json!({"level":level,"count":count}))
            .collect();
        let relations: Vec<_> = self
            .graph
            .relationships
            .counts(id)?
            .into_iter()
            .map(|(relation, count)| json!({"type":relation,"count":count}))
            .collect();
        let mut roots = self.graph.nodes.roots(id, 31)?;
        let truncated = roots.len() > 30;
        roots.truncate(30);
        Ok(json!({"levels":levels,"relations":relations,"roots":roots,"truncated":truncated}))
    }
    fn find(&mut self, query: &str, limit: usize) -> Result<Vec<Value>> {
        self.graph.nodes.find(&self.snapshot.repository_id, query, limit)
    }
    fn node(&mut self, key: &str) -> Result<Value> {
        let id = &self.snapshot.repository_id;
        let mut node = self.graph.nodes.node(id, key)?;
        node["metrics"] = self.graph.metrics.get(id, key)?;
        Ok(node)
    }
    fn children(&mut self, key: &str, limit: usize, offset: usize) -> Result<Vec<Value>> {
        self.graph.nodes.children(&self.snapshot.repository_id, &ChildrenRequest { key, limit, offset })
    }
    fn edges(&mut self, request: &EdgeRequest) -> Result<Vec<Value>> {
        self.graph.relationships.edges(&self.snapshot.repository_id, request)
    }
    fn diagnostics(&mut self, paths: &[String], limit: usize) -> Result<Vec<Value>> {
        self.graph.diagnostics.paths(&self.snapshot.repository_id, paths, limit)
    }
    fn file(&mut self, path: &str) -> Result<Vec<Value>> {
        self.graph.nodes.file(&self.snapshot.repository_id, path)
    }
}
