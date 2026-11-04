//! Exact, raw-source mutation sites. Unknown input fields follow Pydantic's ignore policy.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationSpan {
    pub start_byte: i64,
    pub end_byte: i64,
    pub start_line: i64,
    pub start_column: i64,
    pub end_line: i64,
    pub end_column: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationCallable {
    pub name: String,
    pub span: MutationSpan,
    pub original_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MutationKind {
    Condition,
    Comparison,
    Logical,
    Arithmetic,
    Assignment,
}
impl MutationKind {
    /// Return the lowercase mutation kind used in serialized candidate records.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Condition => "condition",
            Self::Comparison => "comparison",
            Self::Logical => "logical",
            Self::Arithmetic => "arithmetic",
            Self::Assignment => "assignment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationCandidate {
    pub id: String,
    pub path: String,
    pub language: String,
    pub source_hash: String,
    pub kind: MutationKind,
    #[serde(default)]
    pub operator: Option<String>,
    pub span: MutationSpan,
    pub expression_span: MutationSpan,
    pub original_text: String,
    pub expression_text: String,
    #[serde(default)]
    pub callable: Option<MutationCallable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationCandidateAnalysis {
    pub candidates: Vec<MutationCandidate>,
}

/// Deserialize a candidate with object-shaped spans, ignoring unknown fields.
///
/// # Errors
/// Return an error for non-object candidates, spans, or callable records, or
/// fields that fail Serde type/required-field checks. Do not check source contents.
pub fn validate_candidate(value: &serde_json::Value) -> anyhow::Result<MutationCandidate> {
    anyhow::ensure!(value.is_object(), "Mutation candidate must be an object");
    for field in ["span", "expressionSpan"] {
        anyhow::ensure!(value[field].is_object(), "Mutation candidate {field} must be an object");
    }
    if let Some(owner) = value.get("callable").filter(|owner| !owner.is_null()) {
        anyhow::ensure!(owner.is_object() && owner["span"].is_object(), "Mutation callable and span must be objects");
    }
    Ok(serde_json::from_value(value.clone())?)
}
