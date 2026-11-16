use super::{
    navigation::{AdjacencyFilter, ChildrenRequest, EdgeRequest},
    repositories::{GraphDto, RepositoryDto},
};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;

// A scope owns the commit. Dropping an unfinished scope rolls back all capability writes.
pub trait Transaction: Send {
    fn commit(self: Box<Self>) -> Result<()>;
}
pub trait UnitOfWork: Send {
    fn begin(&self) -> Result<Box<dyn Transaction>>;
}
pub trait RepositoryPort: Send {
    fn register(&self, name: &str, location: &str) -> Result<RepositoryDto>;
    fn list(&self) -> Result<Vec<RepositoryDto>>;
    fn exists(&self, id: &str) -> Result<bool>;
    fn resolve(&self, name: &str) -> Result<Option<String>>;
    fn select(&self, name: &str) -> Result<Vec<RepositoryDto>>;
}
pub trait GraphRevisionPort: Send {
    fn get(&self, id: &str) -> Result<Option<GraphDto>>;
    fn imports(&self) -> Result<HashMap<String, i64>>;
    fn save(&self, id: &str, artifact: &Value) -> Result<GraphDto>;
}
pub trait NodePort: Send {
    fn all(&self, id: &str) -> Result<Vec<Value>>;
    fn get(&self, id: &str, key: &str) -> Result<Option<Value>>;
    fn roots(&self, id: &str, limit: usize) -> Result<Vec<Value>>;
    fn find(&self, id: &str, query: &str, limit: usize) -> Result<Vec<Value>>;
    fn children(&self, id: &str, request: &ChildrenRequest<'_>) -> Result<Vec<Value>>;
    fn files_at(&self, id: &str, path: &str, limit: usize) -> Result<Vec<Value>>;
    fn levels(&self, id: &str) -> Result<Vec<(String, i64)>>;
    fn clear(&self, id: &str) -> Result<()>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait FilePort: Send {
    fn get_many(&self, id: &str, keys: &[String]) -> Result<HashMap<String, Value>>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait SymbolPort: Send {
    fn get_many(&self, id: &str, keys: &[String]) -> Result<HashMap<String, Value>>;
    fn count(&self, id: &str) -> Result<i64>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait DecoratorPort: Send {
    fn get(&self, id: &str, key: &str, limit: usize) -> Result<Vec<String>>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait NodeMetricsPort: Send {
    fn all(&self, id: &str) -> Result<HashMap<String, Value>>;
    fn get(&self, id: &str, key: &str) -> Result<Value>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait RelationshipPort: Send {
    fn all(&self, id: &str) -> Result<Vec<Value>>;
    fn counts(&self, id: &str) -> Result<Vec<(String, i64)>>;
    fn adjacency(&self, id: &str, request: &EdgeRequest, filter: AdjacencyFilter<'_>) -> Result<Vec<Value>>;
    fn clear(&self, id: &str) -> Result<()>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait DiagnosticPort: Send {
    fn all(&self, id: &str) -> Result<Vec<Value>>;
    fn paths(&self, id: &str, paths: &[String], limit: usize) -> Result<Vec<Value>>;
    fn errors(&self, id: &str) -> Result<i64>;
    fn replace(&self, id: &str, artifact: &Value) -> Result<()>;
}
pub trait MutationCandidatePort: Send {
    fn all(&self, id: &str) -> Result<Vec<Value>>;
    fn insert(&self, id: &str, artifact: &Value) -> Result<()>;
}
