use crate::domain::repositories::GraphDto;
use crate::domain::storage::GraphRevisionPort;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
pub struct GraphRevisionService {
    repository: Box<dyn GraphRevisionPort>,
}
impl GraphRevisionService {
    pub fn new(repository: impl GraphRevisionPort + 'static) -> Self {
        Self { repository: Box::new(repository) }
    }
    pub(crate) fn get(&self, id: &str) -> Result<Option<GraphDto>> {
        self.repository.get(id)
    }
    pub(crate) fn imports(&self) -> Result<HashMap<String, i64>> {
        self.repository.imports()
    }
    pub(crate) fn save(&self, id: &str, artifact: &Value) -> Result<GraphDto> {
        self.repository.save(id, artifact)
    }
}
