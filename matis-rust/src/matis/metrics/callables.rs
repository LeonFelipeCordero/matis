use super::{adapters::get_metric_adapter, complexity::calculate_complexity};
use crate::domain::models::{ParsedFile, ParsedRepository};
use anyhow::{Result, anyhow};

/// Populate function and method metrics in `file` from retained source, without disk reads.
/// Missing syntax leaves complexity and parameter counts unchanged; line counts are updated.
///
/// # Errors
/// Return an error if the grammar cannot load or parsing fails.
pub fn callable_metrics(file: &mut ParsedFile) -> Result<()> {
    if !file.symbols.iter().any(|symbol| matches!(symbol.kind.as_str(), "function" | "method")) {
        return Ok(());
    }
    let source = String::from_utf8_lossy(&file.source.source).into_owned().into_bytes();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language(&file.source.language)?)?;
    let tree = parser.parse(&source, None).ok_or_else(|| anyhow!("Parsing cancelled for {}", file.source.language))?;
    let adapter = get_metric_adapter(&file.source.language);
    for symbol in file.symbols.iter_mut().filter(|symbol| matches!(symbol.kind.as_str(), "function" | "method")) {
        symbol.line_count =
            Some((symbol.end_line + usize::from(symbol.end_column > 1)).saturating_sub(symbol.start_line));
        let node = symbol
            .end_byte
            .checked_sub(1)
            .and_then(|end| tree.root_node().descendant_for_byte_range(symbol.start_byte, end));
        if let Some(node) = node {
            symbol.complexity = Some(calculate_complexity(node, &adapter.metrics, &source));
            symbol.parameter_count = adapter.parameter_count(node, &source);
        }
    }
    Ok(())
}

/// Enrich each repository file's function and method symbols in place.
///
/// # Errors
/// Propagate parser errors; earlier files can remain enriched after failure.
pub fn add_callable_metrics(repo: &mut ParsedRepository) -> Result<()> {
    for file in &mut repo.files {
        callable_metrics(file)?;
    }
    Ok(())
}
