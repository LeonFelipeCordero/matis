//! Verified independent byte edits. No mutation is composed, applied, or saved.
use super::{
    canonical_json, digest,
    rules::{MutationRule, REGISTRY, validate_rules},
};
use crate::domain::{
    models::SourceFile,
    mutation_candidates::{MutationCandidate, MutationKind, MutationSpan},
};
use crate::parsing::mutation_candidates::extract_candidates;
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use tree_sitter::Parser;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationManifest {
    pub id: String,
    pub candidate_id: String,
    pub source_hash: String,
    pub rule_id: String,
    pub rule_version: i64,
    pub language: String,
    pub path: String,
    pub span: MutationSpan,
    pub original_text: String,
    pub original_bytes: String,
    pub replacement: String,
    pub replacement_bytes: String,
}

fn read_source(root: &Path, candidate: &MutationCandidate) -> Result<SourceFile> {
    let relative = Path::new(&candidate.path);
    let parts: Vec<_> = relative
        .components()
        .filter(|part| *part != std::path::Component::CurDir)
        .map(|part| part.as_os_str().to_string_lossy())
        .collect();
    let normalized = if parts.is_empty() { ".".to_owned() } else { parts.join("/") };
    if relative.is_absolute()
        || relative.components().any(|part| part == std::path::Component::ParentDir)
        || normalized != candidate.path
    {
        bail!("Unsafe mutation path: {}", candidate.path);
    }
    let absolute = root
        .join(relative)
        .canonicalize()
        .map_err(|_| anyhow::anyhow!("Cannot read mutation source: {}", candidate.path))?;
    if !absolute.starts_with(root) {
        bail!("Mutation path escapes root: {}", candidate.path);
    }
    let data =
        std::fs::read(&absolute).map_err(|_| anyhow::anyhow!("Cannot read mutation source: {}", candidate.path))?;
    Ok(SourceFile {
        path: candidate.path.clone(),
        absolute_path: absolute,
        language: candidate.language.clone(),
        source: data,
    })
}

fn binary_spans(source: &SourceFile) -> Result<HashSet<(i64, i64)>> {
    let allowed: &[&str] = match source.language.as_str() {
        "python" | "elixir" => &["binary_operator"],
        "kotlin" => &["additive_expression", "multiplicative_expression"],
        _ => &["binary_expression"],
    };
    let language = tree_sitter_language_pack::get_language(&source.language)?;
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let tree = parser
        .parse(&source.source, None)
        .ok_or_else(|| anyhow::anyhow!("Cannot parse mutation source: {}", source.path))?;
    Ok(crate::syntax::walk(tree.root_node())
        .filter(|node| allowed.contains(&node.kind()))
        .map(|node| (node.start_byte() as i64, node.end_byte() as i64))
        .collect())
}

fn manifest(candidate: &MutationCandidate, rule: &MutationRule, replacement: &str) -> Result<MutationManifest> {
    let mut item = MutationManifest {
        id: String::new(),
        candidate_id: candidate.id.clone(),
        source_hash: candidate.source_hash.clone(),
        rule_id: rule.id.clone(),
        rule_version: rule.version,
        language: candidate.language.clone(),
        path: candidate.path.clone(),
        span: candidate.span.clone(),
        original_text: candidate.original_text.clone(),
        original_bytes: STANDARD.encode(candidate.original_text.as_bytes()),
        replacement: replacement.into(),
        replacement_bytes: STANDARD.encode(replacement.as_bytes()),
    };
    let mut payload = serde_json::to_value(&item)?;
    payload.as_object_mut().unwrap().shift_remove("id");
    item.id = digest(canonical_json(&payload).as_bytes());
    Ok(item)
}

fn rule_edits(candidate: &MutationCandidate, rule: &MutationRule) -> Result<Vec<MutationManifest>> {
    rule.mappings
        .iter()
        .filter(|mapping| {
            mapping.language == candidate.language
                && candidate.operator.as_deref() == Some(&mapping.original)
                && mapping.replacement != candidate.original_text
        })
        .map(|mapping| manifest(candidate, rule, &mapping.replacement))
        .collect()
}

fn edits(
    candidate: &MutationCandidate,
    rules: &[String],
    binary: &HashSet<(i64, i64)>,
) -> Result<Vec<MutationManifest>> {
    if candidate.kind == MutationKind::Arithmetic
        && !binary.contains(&(candidate.expression_span.start_byte, candidate.expression_span.end_byte))
    {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for rule in REGISTRY.iter().filter(|rule| rule.kind == candidate.kind && rules.contains(&rule.id)) {
        result.extend(rule_edits(candidate, rule)?);
    }
    Ok(result)
}

fn verify_replacement(source: &SourceFile, manifest: &MutationManifest, kind: &MutationKind) -> Result<()> {
    let (start, end) = (manifest.span.start_byte as usize, manifest.span.end_byte as usize);
    let mut modified = source.clone();
    modified.source.splice(start..end, manifest.replacement.bytes());
    let sites = extract_candidates(&modified)?;
    let valid = sites.iter().any(|site| {
        site["kind"] == kind.as_str()
            && site["operator"] == manifest.replacement
            && site["span"]["startByte"] == start
            && site["span"]["endByte"] == start + manifest.replacement.len()
    });
    if !valid {
        bail!("Unsafe mutation replacement: {}:{}:{}", manifest.path, start, manifest.rule_id);
    }
    Ok(())
}

struct VerifiedSource {
    source: SourceFile,
    extracted: HashMap<String, MutationCandidate>,
    binary: HashSet<(i64, i64)>,
}

fn source_inventory(root: &Path, candidate: &MutationCandidate) -> Result<VerifiedSource> {
    let source = read_source(root, candidate)?;
    let extracted = extract_candidates(&source)?
        .into_iter()
        .map(|value| {
            let item: MutationCandidate = serde_json::from_value(value)?;
            Ok((item.id.clone(), item))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let binary = if extracted.is_empty() { HashSet::new() } else { binary_spans(&source)? };
    Ok(VerifiedSource { source, extracted, binary })
}

fn candidate_edits(
    candidate: &MutationCandidate,
    verified: &VerifiedSource,
    rules: &[String],
) -> Result<Vec<MutationManifest>> {
    if verified.extracted.get(&candidate.id) != Some(candidate) {
        bail!("Stale or invalid mutation candidate: {}:{}", candidate.path, candidate.id);
    }
    let result = edits(candidate, rules, &verified.binary)?;
    for item in &result {
        verify_replacement(&verified.source, item, &candidate.kind)?;
    }
    Ok(result)
}

fn resolved_root(root: &Path) -> Result<std::path::PathBuf> {
    let absolute = if root.is_absolute() { root.to_path_buf() } else { std::env::current_dir()?.join(root) };
    Ok(resolve_components(&absolute, &mut HashSet::new()))
}

// Path.resolve(strict=False) follows symlinks even when their targets do not exist.
fn resolve_components(path: &Path, active: &mut HashSet<std::path::PathBuf>) -> std::path::PathBuf {
    let mut result = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                result.pop();
            }
            other => {
                result.push(other);
            }
        }
        result = resolve_symlink(result, active);
    }
    result
}

fn resolve_symlink(path: std::path::PathBuf, active: &mut HashSet<std::path::PathBuf>) -> std::path::PathBuf {
    let Ok(target) = std::fs::read_link(&path) else {
        return path;
    };
    if !active.insert(path.clone()) {
        return path;
    }
    let target = if target.is_absolute() { target } else { path.parent().unwrap_or(Path::new("/")).join(target) };
    let resolved = resolve_components(&target, active);
    active.remove(&path);
    resolved
}

type EditKey = (String, String, i64, i64, String);
fn store_edits(output: &mut HashMap<EditKey, MutationManifest>, edits: Vec<MutationManifest>) {
    for item in edits {
        let key = (
            item.path.clone(),
            item.source_hash.clone(),
            item.span.start_byte,
            item.span.end_byte,
            item.replacement.clone(),
        );
        output.entry(key).or_insert(item);
    }
}

/// Verify every candidate against current sources and return sorted, unique edits.
///
/// Resolve `root` and read candidate files beneath it, even for unselected sites
/// or an empty rule selection. Re-extraction checks candidate identity, hashes,
/// spans, text, and grammar provenance. Deduplicate equivalent byte edits and sort
/// by path, byte span, rule, replacement, and ID. Each edit applies independently
/// to its source hash; no source or database writes occur.
///
/// # Errors
/// Reject unknown rules even with no candidates, unsafe paths, unreadable or stale
/// sources, invalid candidates, and replacements that fail syntactic verification.
/// Propagate parsing and serialization failures; never return partial results.
/// Syntactic verification does not guarantee type or runtime validity.
pub fn generate_manifests(
    root: &Path,
    candidates: &[MutationCandidate],
    rules: &[String],
) -> Result<Vec<MutationManifest>> {
    validate_rules(rules)?;
    let root = resolved_root(root)?;
    let mut sorted: Vec<_> = candidates.iter().collect();
    sorted.sort_by(|a, b| (&a.path, &a.language, &a.id).cmp(&(&b.path, &b.language, &b.id)));
    let (mut verified, mut unique) = (HashMap::new(), HashMap::new());
    for candidate in sorted {
        let key = (candidate.path.clone(), candidate.language.clone());
        if !verified.contains_key(&key) {
            verified.insert(key.clone(), source_inventory(&root, candidate)?);
        }
        store_edits(&mut unique, candidate_edits(candidate, &verified[&key], rules)?);
    }
    let mut result: Vec<_> = unique.into_values().collect();
    result.sort_by(|a, b| {
        (&a.path, a.span.start_byte, a.span.end_byte, &a.rule_id, &a.replacement, &a.id).cmp(&(
            &b.path,
            b.span.start_byte,
            b.span.end_byte,
            &b.rule_id,
            &b.replacement,
            &b.id,
        ))
    });
    Ok(result)
}

#[cfg(all(test, unix))]
mod path_tests {
    use super::*;
    #[test]
    fn resolves_missing_targets_and_parent_components() -> Result<()> {
        let dir = tempfile::tempdir()?;
        std::os::unix::fs::symlink("absent/child", dir.path().join("link"))?;
        assert_eq!(resolved_root(&dir.path().join("link/.."))?, dir.path().join("absent"));
        std::os::unix::fs::symlink("loop", dir.path().join("loop"))?;
        assert_eq!(resolved_root(&dir.path().join("loop"))?, dir.path().join("loop"));
        Ok(())
    }
}

/// Validate JSON candidates and return JSON manifests for the selected rule IDs.
///
/// Use [`generate_manifests`] to read and verify current sources under `root`
/// without applying or saving edits. Propagate candidate-schema, generation, and
/// serialization errors; no partial manifest list is returned.
pub fn generate_manifest_values(root: &Path, candidates: &[Value], rules: &[String]) -> Result<Vec<Value>> {
    let candidates =
        candidates.iter().map(crate::domain::mutation_candidates::validate_candidate).collect::<Result<Vec<_>>>()?;
    generate_manifests(root, &candidates, rules)?.into_iter().map(|item| Ok(serde_json::to_value(item)?)).collect()
}
