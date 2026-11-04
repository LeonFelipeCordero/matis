//! Parse in-memory files into source-ordered declarations, imports, and calls.
use super::{
    adapters::{LanguageAdapter, get_adapter},
    structure::{declaration_query, extract_structure, kind_name},
    syntax::{self, DeclarationHeader, ParsedSyntax},
};
use crate::{
    domain::models::{Call, Import, Inheritance, ParsedFile, ParsedRepository, SourceFile, Symbol, file_key},
    syntax::children,
};
use anyhow::Result;
use sha2::{Digest, Sha256};
use tree_sitter::Node;
use tree_sitter_language_pack::{ProcessConfig, StructureItem, process};

#[cfg(test)]
#[path = "../../../tests/unit/parsing/parser_tests.rs"]
mod tests;

/// Test whether the grammar defines any shared call/reference node kind, caching the result.
///
/// # Errors
/// Returns an error if the language-pack grammar cannot be loaded.
pub fn has_call_nodes(language: &str) -> Result<bool> {
    use std::{
        collections::HashMap,
        sync::{Mutex, OnceLock},
    };
    static CACHE: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    if let Some(value) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(language) {
        return Ok(*value);
    }
    let grammar = tree_sitter_language_pack::get_language(language)?;
    let found = (0..grammar.node_kind_count()).any(|index| {
        grammar.node_kind_for_id(index as u16).is_some_and(|kind| syntax::FALLBACK_CALL_TYPES.contains(&kind))
    });
    cache.lock().unwrap_or_else(|e| e.into_inner()).insert(language.to_owned(), found);
    Ok(found)
}

/// Describe the first error or missing node in depth-first source order.
/// Use one-based lines and byte columns; return `None` when no concrete error node is found.
pub fn syntax_error(root: Node<'_>) -> Option<String> {
    if !root.has_error() {
        return None;
    }
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.is_error() || node.is_missing() {
            let point = node.start_position();
            return Some(format!(
                "Syntax error at line {}, column {}; analysis may be incomplete.",
                point.row + 1,
                point.column + 1
            ));
        }
        pending.extend(children(node).into_iter().rev().filter(|child| child.has_error() || child.is_missing()));
    }
    None
}

/// Extract named call/reference records in adapter visitation order from `file` and its root.
/// Map synthetic offsets back to file bytes, use one-based lines and exclusive end offsets,
/// and retain at most 300 evidence characters. The root and adapter must match `file`.
pub fn extract_calls(file: &SourceFile, root: Node<'_>, adapter: LanguageAdapter<'_>) -> Vec<Call> {
    let mut calls = vec![];
    let newlines: Vec<_> =
        file.source.iter().enumerate().filter_map(|(index, byte)| (*byte == b'\n').then_some(index)).collect();
    adapter.visit_calls(root, &file.source, &mut |site, syntax_source| {
        let (name, bare_target, qualifier) = adapter.normalize_call(&site, syntax_source);
        let Some(name) = name.filter(|name| !name.is_empty()) else {
            return;
        };
        let start = (site.node.start_byte() as isize + site.offset) as usize;
        let end = (site.node.end_byte() as isize + site.offset) as usize;
        calls.push(Call {
            file_path: file.path.clone(),
            name,
            start_byte: start,
            end_byte: end,
            line: newlines.partition_point(|offset| *offset < start) + 1,
            evidence: String::from_utf8_lossy(&file.source[start..end]).chars().take(300).collect(),
            bare_target,
            qualifier,
            relationship: adapter.call_relationship(&site).to_owned(),
        });
    });
    calls
}

fn add_structure(file: &mut ParsedFile, item: &StructureItem, (parent, ancestors): (&str, &[String])) -> Result<()> {
    let span = &item.span;
    let name = item.name.as_deref().filter(|name| !name.is_empty()).unwrap_or("<anonymous>").to_owned();
    let mut names = ancestors.to_vec();
    names.push(name.clone());
    let qualified_name = format!("{}::{}", file.source.path, names.join("."));
    let key = format!("symbol:{qualified_name}:{}", span.start_byte);
    let kind = kind_name(&item.kind);
    file.symbols.push(Symbol {
        key: key.clone(),
        file_path: file.source.path.clone(),
        language: file.source.language.clone(),
        name,
        qualified_name,
        kind: kind.clone(),
        parent_key: parent.to_owned(),
        start_byte: span.start_byte,
        end_byte: span.end_byte,
        start_line: span.start_line + 1,
        start_column: span.start_column + 1,
        end_line: span.end_line + 1,
        end_column: span.end_column + 1,
        signature: item.signature.clone(),
        visibility: item.visibility.clone(),
        decorators: item.decorators.clone(),
        complexity: None,
        line_count: None,
        parameter_count: None,
    });
    let adapter = get_adapter(&file.source.language)?;
    let signature = item.signature.as_deref().unwrap_or_default();
    for target_name in
        adapter.inheritance(DeclarationHeader { language: &file.source.language, kind: &kind, signature })
    {
        file.inheritances.push(Inheritance {
            source_key: key.clone(),
            file_path: file.source.path.clone(),
            language: file.source.language.clone(),
            target_name,
            line: span.start_line + 1,
            evidence: signature.to_owned(),
        });
    }
    for child in &item.children {
        add_structure(file, child, (&key, &names))?;
    }
    Ok(())
}

fn parse_tree(source: &SourceFile, adapter: LanguageAdapter<'_>) -> Result<Option<tree_sitter::Tree>> {
    let needed =
        adapter.is_dedicated() || has_call_nodes(&source.language)? || declaration_query(&source.language)?.is_some();
    if needed { Ok(Some(syntax::parse(&source.language, &source.source)?)) } else { Ok(None) }
}

/// Parse in-memory source into graph records without reading the filesystem.
/// Replace invalid UTF-8 for extraction, but hash and retain the original bytes.
/// Record syntax diagnostics for dedicated adapters; leave callable metrics unset.
/// Record lines and columns are one-based; byte offsets are zero-based with exclusive ends.
///
/// # Errors
/// Returns an error for unsupported languages or failures in grammar lookup, parsing,
/// native processing, or declaration query compilation. Syntax errors alone are diagnostics.
pub fn parse_file(source: SourceFile) -> Result<ParsedFile> {
    let adapter = get_adapter(&source.language)?;
    let content_hash = format!("{:x}", Sha256::digest(&source.source));
    let text = String::from_utf8_lossy(&source.source).into_owned();
    let syntax_source = SourceFile { source: text.as_bytes().to_vec(), ..source.clone() };
    let tree = parse_tree(&syntax_source, adapter)?;
    let root = tree.as_ref().map(|tree| tree.root_node());
    let parse_error = if adapter.is_dedicated() { root.and_then(syntax_error) } else { None };
    let calls = root.map(|root| extract_calls(&syntax_source, root, adapter)).unwrap_or_default();
    let result = process(&text, &ProcessConfig::new(source.language.clone()))?;
    let native_imports = result.imports.into_iter().map(|item| (item.source, item.span.start_line + 1)).collect();
    let import_values = match root {
        Some(root) => adapter.imports(root, &syntax_source.source, native_imports),
        None => native_imports,
    };
    let imports = import_values
        .into_iter()
        .map(|(value, line)| Import { file_path: source.path.clone(), source: value, line })
        .collect();
    let structure = match root {
        Some(root) => extract_structure(&ParsedSyntax { source: &syntax_source, root }, &result.structure)?,
        None => result.structure,
    };
    let parent = file_key(&source.path);
    let mut file =
        ParsedFile { source, content_hash, parse_error, calls, imports, symbols: vec![], inheritances: vec![] };
    for item in &structure {
        add_structure(&mut file, item, (&parent, &[]))?;
    }
    Ok(file)
}

/// Parse sources sequentially, preserving input order without deduplication or resolution.
///
/// # Errors
/// Propagates the first `parse_file` error without returning a partial repository.
pub fn parse_files(files: Vec<SourceFile>) -> Result<ParsedRepository> {
    Ok(ParsedRepository { files: files.into_iter().map(parse_file).collect::<Result<Vec<_>>>()? })
}
