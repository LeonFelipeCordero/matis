use anyhow::Result;
use serde_json::{Value, json};
use std::fmt;

pub type Record = Value;
#[derive(Debug)]
pub struct NavigationError(pub String);
impl fmt::Display for NavigationError {
    /// Write the navigation message to `f`, propagating formatter failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for NavigationError {}
/// Wrap `message` as a downcastable navigation error without changing it.
pub fn error(message: impl Into<String>) -> anyhow::Error {
    NavigationError(message.into()).into()
}

#[derive(Clone, Debug)]
pub struct GraphSnapshot {
    pub repository_id: String,
    pub revision: String,
    pub indexed_at: Option<String>,
    pub root: String,
}
impl GraphSnapshot {
    /// Return repository ID, graph revision, and indexing time as response metadata.
    pub fn metadata(&self) -> Value {
        json!({"repositoryId": self.repository_id, "graphRevision": self.revision, "indexedAt": self.indexed_at})
    }
}
#[derive(Clone, Debug)]
pub struct EdgeRequest {
    pub key: String,
    pub direction: String,
    pub relations: Vec<String>,
    pub limit: usize,
}
#[derive(Clone, Debug)]
pub struct TraversalRequest {
    pub key: String,
    pub direction: String,
    pub relations: Vec<String>,
    pub depth: usize,
    pub max_nodes: usize,
    pub max_edges: usize,
}
#[derive(Clone, Debug)]
pub struct PathRequest {
    pub target: String,
    pub traversal: TraversalRequest,
}
#[derive(Clone, Debug)]
pub struct SourceOptions {
    pub max_lines: usize,
    pub start_line: Option<serde_json::Number>,
}

pub trait NavigationReader {
    /// Borrow the repository identity and revision held by this reader.
    fn snapshot(&self) -> &GraphSnapshot;
    /// Read level/relation counts and bounded roots; propagate storage errors.
    fn overview(&mut self) -> Result<Value>;
    /// Return ranked name/path/key matches capped at `limit`, or a read error.
    fn find(&mut self, query: &str, limit: usize) -> Result<Vec<Value>>;
    /// Return details for a stable `key`; fail if the node is absent or storage fails.
    fn node(&mut self, key: &str) -> Result<Value>;
    /// Read up to `limit` direct children after `offset` rows in artifact order.
    /// Return an error if paging cannot be represented by storage or the read fails.
    fn children(&mut self, key: &str, limit: usize, offset: usize) -> Result<Vec<Value>>;
    /// Read bounded adjacency using `request` direction and relation filters; propagate read errors.
    fn edges(&mut self, request: &EdgeRequest) -> Result<Vec<Value>>;
    /// Read up to `limit` diagnostics for exact indexed `paths` in artifact order; propagate read errors.
    fn diagnostics(&mut self, paths: &[String], limit: usize) -> Result<Vec<Value>>;
    /// Read up to two file keys/hashes for exact `path` to detect ambiguity; propagate read errors.
    fn file(&mut self, path: &str) -> Result<Vec<Value>>;
}
