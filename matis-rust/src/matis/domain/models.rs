use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SOURCE_METRIC_NAMES: [&str; 7] = [
    "lineCount",
    "parameterCount",
    "baseCyclomaticComplexity",
    "nestedCyclomaticComplexity",
    "loopNestedComplexity",
    "cyclomaticComplexity",
    "maxDecisionDepth",
];

pub type GraphArtifact = serde_json::Value;
pub type GraphNode = serde_json::Value;
pub type GraphEdge = serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub absolute_path: PathBuf,
    pub language: String,
    pub source: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Complexity {
    pub base_cyclomatic_complexity: usize,
    pub nested_cyclomatic_complexity: usize,
    pub loop_nested_complexity: usize,
    pub cyclomatic_complexity: usize,
    pub max_decision_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub key: String,
    pub file_path: String,
    pub language: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub parent_key: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub signature: Option<String>,
    pub visibility: Option<String>,
    pub decorators: Vec<String>,
    pub complexity: Option<Complexity>,
    pub line_count: Option<usize>,
    pub parameter_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub file_path: String,
    pub source: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Call {
    pub file_path: String,
    pub name: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub line: usize,
    pub evidence: String,
    pub bare_target: bool,
    pub qualifier: Option<String>,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inheritance {
    pub source_key: String,
    pub file_path: String,
    pub language: String,
    pub target_name: String,
    pub line: usize,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub source: SourceFile,
    pub content_hash: String,
    pub parse_error: Option<String>,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub calls: Vec<Call>,
    pub inheritances: Vec<Inheritance>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedRepository {
    pub files: Vec<ParsedFile>,
}
impl ParsedRepository {
    /// Iterate over borrowed declarations in file order without deduplication.
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.files.iter().flat_map(|f| &f.symbols)
    }
    /// Iterate over borrowed unresolved imports in file order without deduplication.
    pub fn imports(&self) -> impl Iterator<Item = &Import> {
        self.files.iter().flat_map(|f| &f.imports)
    }
    /// Iterate over borrowed call sites in file order without deduplication.
    pub fn calls(&self) -> impl Iterator<Item = &Call> {
        self.files.iter().flat_map(|f| &f.calls)
    }
    /// Iterate over borrowed base-type references in file order without deduplication.
    pub fn inheritances(&self) -> impl Iterator<Item = &Inheritance> {
        self.files.iter().flat_map(|f| &f.inheritances)
    }
}

/// Prefix `path` with `file:` without normalizing or checking the path.
pub fn file_key(path: &str) -> String {
    format!("file:{path}")
}
