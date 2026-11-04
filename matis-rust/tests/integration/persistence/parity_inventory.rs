//! Fail if a reference golden family is added without corresponding native coverage.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const LANGUAGES: &[&str] = &["java", "kotlin", "go", "rust", "javascript", "typescript", "elixir", "python", "tsx"];

fn coffee_files(language: &str) -> Vec<String> {
    let mut files: Vec<_> = ["extraction", "analysis", "graph", "cli"]
        .iter()
        .map(|seam| format!("coffee/{language}/{seam}.json"))
        .collect();
    if language != "tsx" {
        files.push(format!("coffee/{language}/navigation.json"));
    }
    files
}

fn mutation_files(language: &str) -> Vec<String> {
    vec![format!("mutations/{language}.json"), format!("mutations/{language}-operators.json")]
}

fn covered_files() -> BTreeSet<String> {
    let coffee = LANGUAGES.iter().flat_map(|language| coffee_files(language));
    let mutations = LANGUAGES.iter().flat_map(|language| mutation_files(language));
    let safety = ["java", "kotlin", "python", "javascript", "typescript"]
        .iter()
        .map(|language| format!("safety_review/{language}.json"));
    let members =
        ["python", "javascript", "typescript", "tsx"].iter().map(|language| format!("script_members/{language}.json"));
    let selection = ["java_callable_selection.json".to_owned(), "kotlin_callable_selection.json".to_owned()];
    coffee.chain(mutations).chain(safety).chain(members).chain(selection).collect()
}

fn files(directory: &Path) -> Vec<PathBuf> {
    fs::read_dir(directory).unwrap().map(Result::unwrap).flat_map(entry_files).collect()
}

fn entry_files(entry: fs::DirEntry) -> Vec<PathBuf> {
    if entry.file_type().unwrap().is_dir() { files(&entry.path()) } else { vec![entry.path()] }
}

#[test]
fn every_reference_golden_has_a_native_suite() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots").canonicalize().unwrap();
    let actual: BTreeSet<_> =
        files(&root).iter().map(|path| path.strip_prefix(&root).unwrap().to_str().unwrap().to_owned()).collect();
    let covered = covered_files();
    assert_eq!(actual, covered, "reference golden inventory changed; extend native tests, not the goldens");
    assert_eq!(covered.len(), 73);
}
