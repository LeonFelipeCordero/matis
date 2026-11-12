//! Discover mutation sites without building metrics, relationships, or a graph.
use crate::parsing::{
    discovery::{discover_files, validate_repository},
    mutation_candidates::extract_candidates,
};
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

/// Read supported sources and extract mutation candidates in full-analysis order.
/// Apply the shared repository validation and ignore rules; preserve exact source
/// bytes for candidate hashes. Do not build reports or read or write storage.
///
/// # Errors
/// Propagate repository validation, discovery, source reading, and extraction errors.
pub fn collect_candidates(root: &Path) -> Result<Vec<Value>> {
    let root = validate_repository(root)?;
    let mut candidates = Vec::new();
    for source in discover_files(&root)? {
        candidates.extend(extract_candidates(&source)?);
    }
    Ok(candidates)
}
