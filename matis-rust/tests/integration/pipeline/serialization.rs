//! Match tests/source_snapshot_fixture.py and tests/project_snapshot_support.py exactly.
use matis::domain::models::ParsedFile;
use serde_json::{Value, json};
use tree_sitter_language_pack::{StructureItem, StructureKind};

pub fn bytes(value: &[u8]) -> Value {
    let hex: String = value.iter().map(|byte| format!("{byte:02x}")).collect();
    json!({"hex": hex})
}

pub fn parsed_file(file: &ParsedFile) -> Value {
    json!({
        "source": {"path": file.source.path, "language": file.source.language, "bytes": bytes(&file.source.source)},
        "content_hash": file.content_hash, "parse_error": file.parse_error, "declarations": file.symbols,
        "calls": file.calls, "imports": file.imports, "inheritances": file.inheritances,
    })
}

fn kind(value: &StructureKind) -> String {
    match value {
        StructureKind::Other(name) if matches!(name.as_str(), "type" | "constructor") => name.clone(),
        StructureKind::Other(_) => serde_json::to_string(value).unwrap().to_lowercase(),
        _ => format!("{value:?}").to_lowercase(),
    }
}

fn structure(item: &StructureItem) -> Value {
    // serde omits optional/empty StructureItem fields; Python dataclasses.fields never does.
    json!({"kind": kind(&item.kind), "name": item.name, "visibility": item.visibility, "span": item.span,
        "children": structures(&item.children), "decorators": item.decorators, "doc_comment": item.doc_comment,
        "signature": item.signature, "body_span": item.body_span})
}

pub fn structures(items: &[StructureItem]) -> Value {
    items.iter().map(structure).collect()
}
