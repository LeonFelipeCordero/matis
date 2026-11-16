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
