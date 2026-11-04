//! The three original structure seams and excluded-language native-only records.
use super::serialization;
use matis::{
    domain::models::SourceFile,
    parsing::{
        discovery::detect_language,
        structure::extract_structure,
        syntax::{self, ParsedSyntax},
    },
};
use serde_json::{Map, Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tree_sitter_language_pack::{ProcessConfig, process};

pub fn source_structures(source: &SourceFile) -> Value {
    let native = process(&String::from_utf8_lossy(&source.source), &ProcessConfig::new(source.language.clone()))
        .unwrap()
        .structure;
    let tree = syntax::parse(&source.language, &source.source).unwrap();
    let syntax = ParsedSyntax { source, root: tree.root_node() };
    let before = serialization::structures(&native);
    let result = json!({"native": before, "tags_only": serialization::structures(&extract_structure(&syntax, &[]).unwrap()),
        "merged": serialization::structures(&extract_structure(&syntax, &native).unwrap())});
    assert_eq!(
        serialization::structures(&native),
        before,
        "structure extraction changed native input: {}",
        source.path
    );
    result
}

fn excluded(path: &Path) -> bool {
    path.is_file()
        && !path.components().any(|part| part.as_os_str() == "__pycache__")
        && detect_language(path).is_none()
}

fn all_paths(directory: &Path) -> Vec<PathBuf> {
    if !directory.is_dir() {
        return vec![];
    }
    fs::read_dir(directory).unwrap().map(Result::unwrap).flat_map(entry_paths).collect()
}

fn entry_paths(entry: fs::DirEntry) -> Vec<PathBuf> {
    if entry.file_type().unwrap().is_dir() { all_paths(&entry.path()) } else { vec![entry.path()] }
}

fn excluded_record(path: &Path) -> Value {
    let content = fs::read(path).unwrap();
    let mut record = json!({"bytes": serialization::bytes(&content)});
    if path.extension().is_some_and(|extension| extension == "h") {
        let tree = syntax::parse("c", &content).unwrap();
        let native = process(&String::from_utf8_lossy(&content), &ProcessConfig::new("c")).unwrap().structure;
        record["native_only"] = json!({"language": "c", "syntax": tree.root_node().to_sexp(),
            "has_error": tree.root_node().has_error(), "native": serialization::structures(&native)});
    }
    record
}

pub fn excluded_sources(root: &Path) -> Value {
    all_paths(&root.join("edge_cases"))
        .into_iter()
        .filter(|path| excluded(path))
        .map(|path| {
            let relative = path.strip_prefix(root).unwrap().to_str().unwrap().to_owned();
            (relative, excluded_record(&path))
        })
        .collect::<Map<_, _>>()
        .into()
}
