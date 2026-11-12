use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(source_files(&path));
        } else if path.extension().is_some_and(|value| value == "rs") {
            files.push(path);
        }
    }
    files
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/matis")
}

fn rust_tree(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language("rust").unwrap()).unwrap();
    let tree = parser.parse(source, None).unwrap();
    assert!(!tree.root_node().has_error(), "Invalid Rust: {source}");
    tree
}

fn boundary_references<'a>(source: &'a str, forbidden: &[&str]) -> BTreeSet<&'a str> {
    let tree = rust_tree(source);
    matis::syntax::walk(tree.root_node())
        .filter(|node| {
            matches!(
                node.kind(),
                "use_declaration" | "scoped_identifier" | "scoped_type_identifier" | "field_expression"
            )
        })
        .flat_map(matis::syntax::walk)
        .filter(|node| matches!(node.kind(), "identifier" | "field_identifier"))
        .map(|node| &source[node.byte_range()])
        .filter(|name| forbidden.contains(name))
        .collect()
}

fn assert_factory_reexports(source: &str) {
    let tree = rust_tree(source);
    for item in matis::syntax::named_children(tree.root_node()) {
        if matches!(item.kind(), "line_comment" | "block_comment") {
            continue;
        }
        assert_eq!(item.kind(), "use_declaration", "Factories must contain only reexports");
        assert!(matis::syntax::named_children(item).iter().any(|node| node.kind() == "visibility_modifier"));
        assert_eq!(
            boundary_references(&source[item.byte_range()], &["bootstrap"]),
            BTreeSet::from(["bootstrap"]),
            "Factories must reexport bootstrap"
        );
    }
}

#[test]
fn storage_construction_stays_in_bootstrap() {
    let native = root();
    for file in source_files(&native) {
        let relative = file.strip_prefix(&native).unwrap();
        // Bootstrap owns composition; persistence owns its internal storage calls.
        if relative == Path::new("bootstrap.rs") || relative.starts_with("persistence") {
            continue;
        }
        let source = fs::read_to_string(&file).unwrap();
        if relative == Path::new("services/factories.rs") {
            assert_factory_reexports(&source);
            continue;
        }
        let forbidden: &[&str] = if relative.starts_with("services") {
            &["persistence", "bootstrap", "factories"]
        } else {
            &["persistence"]
        };
        let references = boundary_references(&source, forbidden);
        assert!(references.is_empty(), "{} crosses the composition boundary: {references:?}", file.display());
    }
}

#[test]
fn mutation_candidates_do_not_depend_on_graph_analysis() {
    for (file, forbidden) in [
        (
            "mutations/candidates.rs",
            &["indexer", "metrics", "parser", "artifact", "report", "services", "persistence", "bootstrap"][..],
        ),
        ("parsing/mutation_candidates.rs", &["mutations"][..]),
        (
            "cli/mutations.rs",
            &[
                "indexer",
                "analyze_folder",
                "analyze_repository",
                "build_artifact_report",
                "build_report",
                "saved_folder_report",
            ][..],
        ),
        (
            "cli/mutation_run.rs",
            &[
                "indexer",
                "analyze_folder",
                "analyze_repository",
                "build_artifact_report",
                "build_report",
                "saved_folder_report",
            ][..],
        ),
    ] {
        let source = fs::read_to_string(root().join(file)).unwrap();
        let references = boundary_references(&source, forbidden);
        assert!(references.is_empty(), "{file} adds unrelated analysis dependencies: {references:?}");
    }
}

#[test]
fn boundary_detector_handles_nested_imports_aliases_types_and_spaced_calls() {
    for source in [
        "use crate::{persistence::{repositories::repository_store as store}, services};",
        "pub use crate::bootstrap as composition;",
        "fn f() { crate :: persistence :: repositories :: repository_store(None); }",
        "type Store = crate::persistence::repositories::RepositoryStore;",
    ] {
        assert_eq!(boundary_references(source, &["persistence", "bootstrap"]).len(), 1, "{source}");
    }
    for source in [
        "use crate::metrics as analysis;",
        "use crate::parsing::{parser as extraction};",
        "fn f() { crate::indexer::analyze_repository(root); }",
        "use crate::services::repositories::analyze_folder as collect;",
        "fn f() { service.saved_folder_report(root); }",
    ] {
        assert_eq!(
            boundary_references(source, &["metrics", "parser", "indexer", "analyze_folder", "saved_folder_report"])
                .len(),
            1,
            "{source}"
        );
    }
    let source = r#"
        // use crate::persistence::repositories;
        pub mod persistence;
        use crate::services::{/* persistence */ repositories};
        fn f() { let _ = "crate::bootstrap::repository_service(None)"; }
    "#;
    assert!(boundary_references(source, &["persistence", "bootstrap"]).is_empty());
}
