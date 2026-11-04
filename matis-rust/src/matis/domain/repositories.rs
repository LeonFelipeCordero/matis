use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Debug)]
pub struct RepositoryError {
    pub message: String,
    pub status: u16,
}
impl RepositoryError {
    /// Store a message and HTTP-style status without validating either value.
    pub fn new(message: impl Into<String>, status: u16) -> Self {
        Self { message: message.into(), status }
    }
}
impl fmt::Display for RepositoryError {
    /// Write the error message to `f`, propagating formatter failures.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for RepositoryError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryDto {
    pub id: String,
    pub name: String,
    pub location: String,
    pub created_at: i64,
    pub imported_at: Option<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GraphStatsDto {
    pub files: i64,
    pub symbols: i64,
    pub modules: i64,
    pub types: i64,
    pub callables: i64,
    pub nodes: i64,
    pub edges: i64,
    pub external_nodes: i64,
    pub parse_errors: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDto {
    pub repository_id: String,
    pub graph_revision: String,
    pub schema_version: Option<i64>,
    pub indexed_at: Option<String>,
    pub root_name: Option<String>,
    pub root_path: Option<String>,
    pub imported_at: i64,
    pub stats: GraphStatsDto,
}
pub type ReportFactsDto = Value;

pub trait RepositoryProtocol {
    /// Persist a name/location registration without indexing; return its DTO.
    /// Return an error if registration cannot be stored, including name conflicts.
    fn register(&mut self, name: &str, location: &str) -> anyhow::Result<RepositoryDto>;
    /// Return registrations with their latest graph import times, or a storage error.
    fn list(&mut self) -> anyhow::Result<Vec<RepositoryDto>>;
    /// Check whether a repository ID is registered, or return a storage error.
    fn exists(&mut self, identifier: &str) -> anyhow::Result<bool>;
    /// Resolve a case-insensitive name to an ID; return `None` if absent or an error on storage failure.
    fn resolve(&mut self, name: &str) -> anyhow::Result<Option<String>>;
    /// Return the ID's saved graph summary, `None` without a graph, or a storage error.
    fn graph(&mut self, identifier: &str) -> anyhow::Result<Option<GraphDto>>;
    /// Return one saved revision's report inputs, `None` without a graph, or a read error.
    fn report_facts(&mut self, identifier: &str) -> anyhow::Result<Option<ReportFactsDto>>;
    /// Atomically replace a registered ID's graph with `artifact` and return its summary.
    /// Validation or storage errors must preserve the previous saved graph.
    fn save(&mut self, identifier: &str, artifact: &Value) -> anyhow::Result<GraphDto>;
    /// Atomically persist a name/location registration and `artifact`; return its summary.
    /// Validation or storage errors must leave neither a registration nor a partial graph.
    fn save_new(&mut self, name: &str, location: &str, artifact: &Value) -> anyhow::Result<GraphDto>;
}
