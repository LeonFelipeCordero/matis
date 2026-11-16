use crate::domain::storage::NodeMetricsPort;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
pub struct NodeMetricsService {
    repository: Box<dyn NodeMetricsPort>,
}
impl NodeMetricsService {
    pub fn new(repository: impl NodeMetricsPort + 'static) -> Self {
        Self { repository: Box::new(repository) }
    }
    pub(crate) fn all(&self, id: &str) -> Result<HashMap<String, Value>> {
        self.repository.all(id)
    }
    pub(crate) fn get(&self, id: &str, key: &str) -> Result<Value> {
        self.repository.get(id, key)
    }
    pub(crate) fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        self.repository.insert(id, artifact)
    }
}
