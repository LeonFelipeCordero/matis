//! Analyze repositories into in-memory graph artifacts.
use std::path::Path;

use anyhow::Result;

use crate::domain::models::GraphArtifact;
use crate::metrics::{artifact::build_artifact, callables::add_callable_metrics, resolver::resolve_connections};
use crate::parsing::{
    discovery::{discover_files, validate_repository},
    parser::parse_files_with,
    syntax::ParsedSyntax,
};

/// Read a local repository and build an in-memory graph with metrics, relationships, and diagnostics.
/// Do not open a database or write an output file; Git is not required.
///
/// # Arguments
/// * `root` - Relative or absolute directory to validate and resolve before discovery.
///
/// # Errors
/// Propagate invalid-root, path/ignore/source reading, parsing, metric, resolution,
/// and artifact-construction failures. File parse diagnostics may also be included in the graph.
pub fn analyze_repository(root: &Path) -> Result<GraphArtifact> {
    analyze_repository_with(root, |_| {})
}

pub(crate) fn analyze_repository_with(root: &Path, observe: impl FnMut(ParsedSyntax<'_>)) -> Result<GraphArtifact> {
    let root = validate_repository(root)?;
    let mut repository = parse_files_with(discover_files(&root)?, observe)?;
    add_callable_metrics(&mut repository)?;
    let graph = resolve_connections(&repository)?;
    build_artifact(&root, &repository, graph)
}
