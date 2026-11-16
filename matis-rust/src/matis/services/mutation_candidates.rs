use crate::domain::storage::MutationCandidatePort;
use anyhow::Result;
use serde_json::Value;
pub struct MutationCandidateService {
    repository: Box<dyn MutationCandidatePort>,
}
impl MutationCandidateService {
    pub fn new(repository: impl MutationCandidatePort + 'static) -> Self {
        Self { repository: Box::new(repository) }
    }
    pub(crate) fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.repository.all(id)
    }
    pub(crate) fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        self.repository.insert(id, artifact)
    }
}
