use crate::domain::storage::DiagnosticPort;
use anyhow::Result;
use serde_json::Value;
pub struct DiagnosticService {
    repository: Box<dyn DiagnosticPort>,
}
impl DiagnosticService {
    pub fn new(repository: impl DiagnosticPort + 'static) -> Self {
        Self { repository: Box::new(repository) }
    }
    pub(crate) fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.repository.all(id)
    }
    pub(crate) fn paths(&self, id: &str, paths: &[String], limit: usize) -> Result<Vec<Value>> {
        self.repository.paths(id, paths, limit)
    }
    pub(crate) fn errors(&self, id: &str) -> Result<i64> {
        self.repository.errors(id)
    }
    pub(crate) fn replace(&self, id: &str, artifact: &Value) -> Result<()> {
        self.repository.replace(id, artifact)
    }
}
