use super::{
    cycles::circular_dependencies,
    dependency_metrics::{METRIC_NAMES, dependency_metrics},
    resolver::ResolvedGraph,
};
use crate::domain::models::{GraphArtifact, ParsedRepository, Symbol, file_key};
use crate::parsing::mutation_candidates::extract_candidates;
use anyhow::{Result, anyhow, ensure};
use chrono::Utc;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const LEVELS: [&str; 4] = ["module", "file", "type", "callable"];

fn parent(path: &str) -> &str {
    path.rsplit_once('/').map(|(parent, _)| parent).filter(|p| !p.is_empty()).unwrap_or(".")
}
fn module_key(path: &str) -> String {
    format!("module:{path}")
}
fn symbol_level(kind: &str) -> &str {
    match kind {
        "module" | "namespace" | "package" => "module",
        "class" | "enum" | "interface" | "struct" | "trait" | "type" => "type",
        "constructor" | "function" | "method" | "procedure" => "callable",
        _ => "symbol",
    }
}

struct SourceLocation<'a> {
    path: &'a str,
    line: usize,
    evidence: Option<String>,
}
struct Contents {
    nodes: Vec<Value>,
    edges: Vec<Value>,
    diagnostics: Vec<Value>,
}

fn contains(source: &str, target: &str, location: SourceLocation<'_>) -> Value {
    json!({"sourceStableKey": source, "targetStableKey": target, "type": "CONTAINS",
        "confidence": "EXTRACTED", "path": location.path, "line": location.line, "evidence": location.evidence})
}
fn directory_ancestors(path: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut current = parent(path);
    while current != "." {
        paths.insert(current.to_owned());
        current = parent(current);
    }
    paths
}

fn add_directories(root: &Path, repo: &ParsedRepository, contents: &mut Contents) {
    let mut paths = BTreeSet::from([".".to_owned()]);
    for file in &repo.files {
        paths.extend(directory_ancestors(&file.source.path));
    }
    let mut paths: Vec<_> = paths.into_iter().collect();
    paths.sort_by_key(|path| (path.matches('/').count(), path.clone()));
    let root_name = root.file_name().unwrap_or_default().to_string_lossy();
    for path in paths {
        let parent_key = if path == "." { "repository".to_owned() } else { module_key(parent(&path)) };
        let name = if path == "." { root_name.as_ref() } else { path.rsplit('/').next().unwrap() };
        let qualified = if path == "." { root_name.to_string() } else { path.replace('/', ".") };
        contents.nodes.push(
            json!({"stableKey": module_key(&path), "type": "module", "kind": "directory", "level": "module",
            "language": null, "name": name, "qualifiedName": qualified, "path": path,
            "parentStableKey": parent_key, "properties": {}}),
        );
        contents.edges.push(contains(
            &parent_key,
            &module_key(&path),
            SourceLocation { path: &path, line: 1, evidence: None },
        ));
    }
}

fn add_files(repo: &ParsedRepository, contents: &mut Contents) {
    for file in &repo.files {
        let source = &file.source;
        let parent_key = module_key(parent(&source.path));
        let line_count = source.source.iter().filter(|byte| **byte == b'\n').count()
            + usize::from(!source.source.is_empty() && !source.source.ends_with(b"\n"));
        contents.nodes.push(
            json!({"stableKey": file_key(&source.path), "type": "file", "kind": "file", "level": "file",
            "language": source.language, "name": source.absolute_path.file_name().unwrap_or_default().to_string_lossy(),
            "qualifiedName": source.path, "path": source.path, "parentStableKey": parent_key,
            "properties": {"bytes": source.source.len(), "lineCount": line_count, "sha256": file.content_hash}}),
        );
        contents.edges.push(contains(
            &parent_key,
            &file_key(&source.path),
            SourceLocation { path: &source.path, line: 1, evidence: None },
        ));
        if let Some(error) = &file.parse_error
            && !error.is_empty()
        {
            contents
                .diagnostics
                .push(json!({"severity": "error", "path": source.path, "language": source.language, "message": error}));
        }
    }
}

fn symbol_node(symbol: &Symbol) -> Result<Value> {
    let mut properties = json!({"visibility": symbol.visibility, "decorators": symbol.decorators});
    if let Some(value) = symbol.line_count {
        properties["lineCount"] = json!(value);
    }
    if let Some(value) = symbol.parameter_count {
        properties["parameterCount"] = json!(value);
    }
    if let Some(value) = &symbol.complexity {
        properties.as_object_mut().unwrap().extend(serde_json::to_value(value)?.as_object().unwrap().clone());
    }
    Ok(json!({"stableKey": symbol.key, "type": symbol.kind, "kind": symbol.kind, "level": symbol_level(&symbol.kind),
        "language": symbol.language, "name": symbol.name, "qualifiedName": symbol.qualified_name, "path": symbol.file_path,
        "parentStableKey": symbol.parent_key, "startLine": symbol.start_line, "startColumn": symbol.start_column,
        "endLine": symbol.end_line, "endColumn": symbol.end_column, "signature": symbol.signature, "properties": properties}))
}

fn add_hierarchy(nodes: &mut [Value]) -> Result<()> {
    let parents: BTreeMap<_, _> = nodes
        .iter()
        .map(|node| {
            (node["stableKey"].as_str().unwrap().to_owned(), node["parentStableKey"].as_str().map(str::to_owned))
        })
        .collect();
    for node in nodes {
        let ancestors = ancestor_keys(node, &parents)?;
        node["depth"] = json!(ancestors.len());
        node["ancestorStableKeys"] = json!(ancestors);
    }
    Ok(())
}

fn ancestor_keys(node: &Value, parents: &BTreeMap<String, Option<String>>) -> Result<Vec<String>> {
    let mut ancestors = Vec::new();
    let mut current = node["parentStableKey"].as_str().map(str::to_owned);
    let mut visited = BTreeSet::new();
    while let Some(key) = current {
        ensure!(visited.insert(key.clone()), "Graph hierarchy contains a parent cycle");
        ancestors.push(key.clone());
        current = parents.get(&key).cloned().flatten();
    }
    ancestors.reverse();
    Ok(ancestors)
}

fn enrich_edges(edges: &mut [Value], nodes: &[Value]) -> Result<()> {
    let lookup: BTreeMap<_, _> = nodes.iter().map(|node| (node["stableKey"].as_str().unwrap(), node)).collect();
    for edge in edges {
        enrich_edge(edge, &lookup)?;
    }
    Ok(())
}
fn enrich_edge(edge: &mut Value, lookup: &BTreeMap<&str, &Value>) -> Result<()> {
    for endpoint in ["source", "target"] {
        let key = edge[format!("{endpoint}StableKey")].as_str().unwrap_or("");
        let node = lookup.get(key).ok_or_else(|| anyhow!("Graph edge references missing {endpoint}: {key}"))?;
        enrich_endpoint(edge, endpoint, node);
    }
    Ok(())
}
fn enrich_endpoint(edge: &mut Value, endpoint: &str, node: &Value) {
    for (field, suffix) in [("type", "Type"), ("kind", "Kind"), ("level", "Level")] {
        edge[format!("{endpoint}{suffix}")] = node[field].clone();
    }
}

fn projected_key(key: &str, level: &str, lookup: &BTreeMap<&str, &Value>) -> Option<String> {
    let mut current = lookup.get(key).copied();
    let mut visited = BTreeSet::new();
    while let Some(node) = current {
        let key = node["stableKey"].as_str()?;
        if node["level"] == level {
            return Some(key.to_owned());
        }
        if !visited.insert(key) {
            return None;
        }
        current = node["parentStableKey"].as_str().and_then(|key| lookup.get(key).copied());
    }
    None
}

/// Project graph dependencies onto module, file, type, and callable ancestors.
/// Exclude containment and same-node projections; aggregate counts by endpoint pair.
/// `nodes` must provide the hierarchy and stable keys used by `edges`.
pub fn build_dependency_views(nodes: &[Value], edges: &[Value]) -> Value {
    let mut views = json!({});
    for level in LEVELS {
        views[level] = level_view(nodes, edges, level);
    }
    views
}
fn level_view(nodes: &[Value], edges: &[Value], level: &str) -> Value {
    let lookup = nodes.iter().map(|node| (node["stableKey"].as_str().unwrap_or(""), node)).collect();
    let keys: Vec<_> =
        nodes.iter().filter(|node| node["level"] == level).map(|node| node["stableKey"].clone()).collect();
    let projected: Vec<_> = projected_relationships(edges, level, &lookup)
        .into_iter()
        .map(|((source, target), edges)| {
            json!({"sourceStableKey": source, "targetStableKey": target, "type": "DEPENDS_ON", "level": level,
            "count": edges.len(), "relationshipCounts": relationship_counts(&edges, "type"),
            "confidenceCounts": relationship_counts(&edges, "confidence")})
        })
        .collect();
    json!({"nodes": keys, "edges": projected})
}
fn projected_relationships<'a>(
    edges: &'a [Value],
    level: &str,
    lookup: &BTreeMap<&str, &Value>,
) -> BTreeMap<(String, String), Vec<&'a Value>> {
    let mut relationships: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for edge in edges {
        if edge["type"] == "CONTAINS" {
            continue;
        }
        let source = projected_key(edge["sourceStableKey"].as_str().unwrap_or(""), level, lookup);
        let target = projected_key(edge["targetStableKey"].as_str().unwrap_or(""), level, lookup);
        let (Some(source), Some(target)) = (source, target) else {
            continue;
        };
        if source == target {
            continue;
        }
        relationships.entry((source, target)).or_default().push(edge);
    }
    relationships
}

fn relationship_counts(edges: &[&Value], field: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for edge in edges {
        *counts.entry(edge[field].as_str().unwrap_or("").to_owned()).or_default() += 1;
    }
    counts
}

fn add_dependency_metrics(nodes: &mut [Value], views: &Value) {
    let metrics: BTreeMap<_, _> = LEVELS.iter().flat_map(|level| dependency_metrics(&views[level])).collect();
    for node in nodes {
        if let Some(values) = metrics.get(node["stableKey"].as_str().unwrap_or("")) {
            node["properties"].as_object_mut().unwrap().extend(values.as_object().unwrap().clone());
        }
    }
}

/// Assemble a validated version-2 graph artifact from `repo` and resolved `graph`.
/// Use `root` for repository metadata and record the current UTC indexing time.
/// Include hierarchy, dependency metrics, runtime import cycles, and mutation candidates;
/// return the artifact without writing it to disk.
///
/// # Errors
/// Return serialization, hierarchy, missing-reference, mutation extraction, or validation errors.
pub fn build_artifact(root: &Path, repo: &ParsedRepository, graph: ResolvedGraph) -> Result<GraphArtifact> {
    let name = root.file_name().unwrap_or_default().to_string_lossy();
    let nodes =
        vec![json!({"stableKey": "repository", "type": "repository", "kind": "repository", "level": "repository",
        "language": null, "name": name, "qualifiedName": name, "path": ".", "parentStableKey": null,
        "properties": {"absolutePath": root.to_string_lossy()}})];
    let mut contents = Contents { nodes, edges: Vec::new(), diagnostics: Vec::new() };
    add_directories(root, repo, &mut contents);
    add_files(repo, &mut contents);
    let Contents { mut nodes, mut edges, diagnostics } = contents;
    for symbol in repo.symbols() {
        nodes.push(symbol_node(symbol)?);
        edges.push(contains(
            &symbol.parent_key,
            &symbol.key,
            SourceLocation {
                path: &symbol.file_path,
                line: symbol.start_line,
                evidence: Some(format!("{} contains {}", symbol.parent_key, symbol.name)),
            },
        ));
    }
    for external in &graph.external_nodes {
        nodes.push(json!({"stableKey": external.key, "type": "external", "kind": external.kind, "level": "external",
            "language": external.language, "name": external.name.rsplit('.').next().unwrap_or(""),
            "qualifiedName": format!("external:{}:{}", external.language, external.name), "path": null,
            "parentStableKey": "repository", "properties": {}}));
    }
    for edge in &graph.connections {
        let mut value = json!({"sourceStableKey": edge.source_key, "targetStableKey": edge.target_key, "type": edge.type_,
            "confidence": edge.confidence, "path": edge.path, "line": edge.line, "evidence": edge.evidence});
        if edge.type_only {
            value["typeOnly"] = json!(true);
        }
        edges.push(value);
    }
    add_hierarchy(&mut nodes)?;
    enrich_edges(&mut edges, &nodes)?;
    let views = build_dependency_views(&nodes, &edges);
    add_dependency_metrics(&mut nodes, &views);
    let mut mutation_candidates = Vec::new();
    for file in &repo.files {
        mutation_candidates.extend(extract_candidates(&file.source)?);
    }
    let languages: BTreeSet<_> = repo.files.iter().map(|file| &file.source.language).collect();
    let artifact = json!({"schemaVersion": 2, "repository": {"path": root.to_string_lossy(), "name": name,
        "indexedAt": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false), "languages": languages},
        "stats": {"files": repo.files.len(), "symbols": repo.symbols().count(), "modules": views["module"]["nodes"].as_array().unwrap().len(),
            "types": views["type"]["nodes"].as_array().unwrap().len(), "callables": views["callable"]["nodes"].as_array().unwrap().len(),
            "nodes": nodes.len(), "edges": edges.len(), "externalNodes": graph.external_nodes.len(), "parseErrors": diagnostics.len()},
        "circularDependencies": circular_dependencies(&nodes, &edges), "nodes": nodes, "edges": edges,
        "views": views, "diagnostics": diagnostics, "mutationCandidates": mutation_candidates});
    validate_artifact(&artifact)?;
    Ok(artifact)
}

/// Check unique node keys, parent/edge references, and dependency property values.
/// Do not modify `artifact`.
///
/// # Errors
/// Reject malformed arrays, missing references, duplicate keys, or invalid dependency metrics.
pub fn validate_references(artifact: &Value) -> Result<()> {
    let nodes = artifact["nodes"].as_array().ok_or_else(|| anyhow!("nodes must be an array"))?;
    let keys: BTreeSet<_> = nodes.iter().filter_map(|node| node["stableKey"].as_str()).collect();
    ensure!(keys.len() == nodes.len(), "Graph artifact contains duplicate stable node keys");
    for node in nodes {
        validate_node_references(node, &keys)?;
    }
    for edge in required_array(artifact, "edges")? {
        validate_edge_references(edge, &keys)?;
    }
    Ok(())
}
fn validate_node_references(node: &Value, keys: &BTreeSet<&str>) -> Result<()> {
    validate_dependency_properties(&node["properties"])?;
    if let Some(parent) = node["parentStableKey"].as_str().filter(|key| !key.is_empty()) {
        ensure!(keys.contains(parent), "Graph node references missing parent: {parent}");
    }
    Ok(())
}
fn validate_dependency_properties(properties: &Value) -> Result<()> {
    for metric in METRIC_NAMES {
        if let Some(value) = properties.get(metric) {
            ensure!(value.as_u64().is_some(), "Node property {metric} must be a nonnegative integer");
        }
    }
    let value = &properties["instability"];
    ensure!(
        value.is_null()
            || (value.is_f64() && value.as_f64().is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v))),
        "Node property instability must be None or a finite float between 0 and 1"
    );
    Ok(())
}
fn validate_edge_references(edge: &Value, keys: &BTreeSet<&str>) -> Result<()> {
    for endpoint in ["source", "target"] {
        let key = edge[format!("{endpoint}StableKey")].as_str().unwrap_or("");
        ensure!(keys.contains(key), "Graph edge references missing {endpoint}: {key}");
    }
    Ok(())
}

/// Validate version-2 artifact fields, optional sections, and graph references without modifying them.
///
/// # Errors
/// Return an error for unsupported fields, invalid types or values, or broken references.
pub fn validate_artifact(artifact: &Value) -> Result<()> {
    reject_extra(
        artifact,
        &[
            "schemaVersion",
            "repository",
            "stats",
            "nodes",
            "edges",
            "views",
            "diagnostics",
            "circularDependencies",
            "mutationCandidates",
        ],
    )?;
    ensure!(artifact["schemaVersion"].as_u64() == Some(2), "schemaVersion must be 2");
    validate_metadata(&artifact["repository"])?;
    let stats = artifact["stats"].as_object().ok_or_else(|| anyhow!("stats must be an object"))?;
    for value in stats.values() {
        ensure!(value.as_u64().is_some(), "stats must be nonnegative integers");
    }
    for node in required_array(artifact, "nodes")? {
        validate_node(node)?;
    }
    for edge in required_array(artifact, "edges")? {
        validate_edge(edge)?;
    }
    let views = artifact["views"].as_object().ok_or_else(|| anyhow!("views must be an object"))?;
    for view in views.values() {
        validate_view(view)?;
    }
    for diagnostic in required_array(artifact, "diagnostics")? {
        validate_diagnostic(diagnostic)?;
    }
    validate_optional_sections(artifact)?;
    validate_references(artifact)
}
fn validate_diagnostic(diagnostic: &Value) -> Result<()> {
    reject_extra(diagnostic, &["severity", "path", "language", "message"])?;
    required_strings(diagnostic, &["path", "language", "message"])?;
    ensure!(diagnostic["severity"] == "error", "severity must be error");
    Ok(())
}
fn validate_optional_sections(artifact: &Value) -> Result<()> {
    if let Some(cycles) = artifact.get("circularDependencies").filter(|value| !value.is_null()) {
        validate_cycles(cycles)?;
    }
    if artifact.get("mutationCandidates").is_some() {
        ensure!(
            required_array(artifact, "mutationCandidates")?.iter().all(Value::is_object),
            "mutationCandidates must contain objects"
        );
    }
    Ok(())
}

fn reject_extra(value: &Value, fields: &[&str]) -> Result<()> {
    for name in value.as_object().ok_or_else(|| anyhow!("expected an object"))?.keys() {
        ensure!(fields.contains(&name.as_str()), "{name}: Extra inputs are not permitted [type=extra_forbidden]");
    }
    Ok(())
}
fn required_strings(value: &Value, fields: &[&str]) -> Result<()> {
    for name in fields {
        ensure!(value.get(name).is_some_and(Value::is_string), "{name} must be a string");
    }
    Ok(())
}
fn required_array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>> {
    value[field].as_array().ok_or_else(|| anyhow!("{field} must be an array"))
}
fn string_array(value: &Value, field: &str) -> Result<()> {
    ensure!(required_array(value, field)?.iter().all(Value::is_string), "{field} must contain strings");
    Ok(())
}
fn nullable_string(value: &Value, field: &str) -> Result<()> {
    ensure!(value.get(field).is_some_and(|v| v.is_null() || v.is_string()), "{field} must be a string or null");
    Ok(())
}
fn valid_level(value: &Value) -> bool {
    value.as_str().is_some_and(|level| {
        ["repository", "module", "file", "type", "callable", "symbol", "external"].contains(&level)
    })
}
fn validate_metadata(value: &Value) -> Result<()> {
    reject_extra(value, &["path", "name", "indexedAt", "languages"])?;
    for field in ["path", "name", "indexedAt"] {
        if value.get(field).is_some() {
            required_strings(value, &[field])?;
        }
    }
    if value.get("languages").is_some() {
        string_array(value, "languages")?;
    }
    Ok(())
}
fn validate_node(value: &Value) -> Result<()> {
    reject_extra(
        value,
        &[
            "stableKey",
            "type",
            "kind",
            "level",
            "language",
            "name",
            "qualifiedName",
            "path",
            "parentStableKey",
            "startLine",
            "startColumn",
            "endLine",
            "endColumn",
            "signature",
            "properties",
            "depth",
            "ancestorStableKeys",
        ],
    )?;
    required_strings(value, &["stableKey", "type", "kind", "name", "qualifiedName"])?;
    for field in ["language", "path", "parentStableKey"] {
        nullable_string(value, field)?;
    }
    if value.get("signature").is_some() {
        nullable_string(value, "signature")?;
    }
    for field in ["startLine", "startColumn", "endLine", "endColumn"] {
        if let Some(number) = value.get(field) {
            ensure!(number.as_u64().is_some_and(|v| v > 0), "{field} must be positive");
        }
    }
    ensure!(valid_level(&value["level"]), "invalid node level");
    ensure!(value["properties"].is_object(), "properties must be an object");
    ensure!(value["depth"].as_u64().is_some(), "depth must be nonnegative");
    string_array(value, "ancestorStableKeys")
}
fn validate_edge(value: &Value) -> Result<()> {
    reject_extra(
        value,
        &[
            "sourceStableKey",
            "targetStableKey",
            "type",
            "confidence",
            "path",
            "line",
            "evidence",
            "typeOnly",
            "sourceType",
            "sourceKind",
            "sourceLevel",
            "targetType",
            "targetKind",
            "targetLevel",
        ],
    )?;
    required_strings(
        value,
        &["sourceStableKey", "targetStableKey", "path", "sourceType", "sourceKind", "targetType", "targetKind"],
    )?;
    ensure!(
        ["CONTAINS", "CALLS", "REFERENCES", "IMPORTS", "INHERITS"].iter().any(|kind| value["type"] == *kind),
        "invalid relationship"
    );
    ensure!(value["confidence"] == "EXTRACTED" || value["confidence"] == "INFERRED", "invalid confidence");
    ensure!(value["line"].as_u64().is_some_and(|v| v > 0), "line must be positive");
    nullable_string(value, "evidence")?;
    ensure!(valid_level(&value["sourceLevel"]) && valid_level(&value["targetLevel"]), "invalid edge level");
    if let Some(value) = value.get("typeOnly") {
        ensure!(value.is_boolean(), "typeOnly must be boolean");
    }
    Ok(())
}
fn validate_view(value: &Value) -> Result<()> {
    reject_extra(value, &["nodes", "edges"])?;
    string_array(value, "nodes")?;
    for edge in required_array(value, "edges")? {
        validate_projected_edge(edge)?;
    }
    Ok(())
}
fn validate_projected_edge(value: &Value) -> Result<()> {
    reject_extra(
        value,
        &["sourceStableKey", "targetStableKey", "type", "level", "count", "relationshipCounts", "confidenceCounts"],
    )?;
    required_strings(value, &["sourceStableKey", "targetStableKey"])?;
    ensure!(value["type"] == "DEPENDS_ON", "projected type must be DEPENDS_ON");
    ensure!(value["level"].as_str().is_some_and(|v| LEVELS.contains(&v)), "invalid projected level");
    ensure!(value["count"].as_u64().is_some_and(|v| v > 0), "count must be positive");
    for field in ["relationshipCounts", "confidenceCounts"] {
        validate_counts(&value[field], field)?;
    }
    Ok(())
}
fn validate_counts(value: &Value, field: &str) -> Result<()> {
    let counts = value.as_object().ok_or_else(|| anyhow!("{field} must be an object"))?;
    let allowed = if field == "relationshipCounts" {
        &["CONTAINS", "CALLS", "REFERENCES", "IMPORTS", "INHERITS"][..]
    } else {
        &["EXTRACTED", "INFERRED"][..]
    };
    for (key, count) in counts {
        ensure!(allowed.contains(&key.as_str()) && count.as_u64().is_some_and(|v| v > 0), "invalid {field}");
    }
    Ok(())
}
fn validate_cycles(value: &Value) -> Result<()> {
    reject_extra(value, &["version", "groups"])?;
    ensure!(value["version"].as_i64().is_some(), "cycle version must be an integer");
    for group in required_array(value, "groups")? {
        validate_cycle_group(group)?;
    }
    Ok(())
}
fn validate_cycle_group(group: &Value) -> Result<()> {
    reject_extra(group, &["id", "nodeStableKeys", "exampleCycle", "edges"])?;
    required_strings(group, &["id"])?;
    string_array(group, "nodeStableKeys")?;
    string_array(group, "exampleCycle")?;
    ensure!(required_array(group, "edges")?.iter().all(Value::is_object), "cycle edges must contain objects");
    Ok(())
}
