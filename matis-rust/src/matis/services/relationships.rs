use crate::domain::navigation::{AdjacencyFilter, EdgeRequest};
use crate::domain::storage::RelationshipPort;
use anyhow::Result;
use serde_json::Value;
use std::collections::BTreeMap;
pub struct RelationshipService {
    repository: Box<dyn RelationshipPort>,
}
impl RelationshipService {
    pub fn new(repository: impl RelationshipPort + 'static) -> Self {
        Self { repository: Box::new(repository) }
    }
    pub(crate) fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.repository.all(id)
    }
    pub(crate) fn counts(&self, id: &str) -> Result<Vec<(String, i64)>> {
        self.repository.counts(id)
    }
    pub(crate) fn clear(&self, id: &str) -> Result<()> {
        self.repository.clear(id)
    }
    pub(crate) fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        self.repository.insert(id, artifact)
    }
    pub(crate) fn edges(&self, id: &str, request: &EdgeRequest) -> Result<Vec<Value>> {
        let directions = match request.direction.as_str() {
            "both" => vec![true, false],
            "out" => vec![true],
            _ => vec![false],
        };
        let mut selected = BTreeMap::new();
        for (outgoing, relation) in directions
            .into_iter()
            .flat_map(|outgoing| request.relations.iter().map(move |relation| (outgoing, relation)))
        {
            selected.extend(
                self.repository
                    .adjacency(id, request, AdjacencyFilter { outgoing, relation })?
                    .into_iter()
                    .map(|e| (e["ordinal"].as_i64().unwrap(), e)),
            );
        }
        Ok(selected.into_values().take(request.limit).collect())
    }
}
