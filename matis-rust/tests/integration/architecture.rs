use std::{
    fs,
    path::{Path, PathBuf},
};

fn source_files(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(source_files(&path, extension));
        } else if path.extension().is_some_and(|value| value == extension) {
            files.push(path);
        }
    }
    files
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/matis")
}

#[test]
fn storage_construction_stays_in_service_factories() {
    let native = root();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language("rust").unwrap()).unwrap();
    let forbidden = regex::Regex::new(r"\bpersistence\b").unwrap();
    for layer in ["cli", "domain", "parsing", "metrics", "services"] {
        assert_layer(&native.join(layer), (&mut parser, &forbidden));
    }
}

fn assert_layer(directory: &Path, (parser, forbidden): (&mut tree_sitter::Parser, &regex::Regex)) {
    for file in source_files(directory, "rs") {
        if file.ends_with("services/factories.rs") {
            continue;
        }
        assert_file(&file, (parser, forbidden));
    }
}

fn assert_file(file: &Path, (parser, forbidden): (&mut tree_sitter::Parser, &regex::Regex)) {
    let source = fs::read_to_string(file).unwrap();
    let tree = parser.parse(&source, None).unwrap();
    let imports = matis::syntax::walk(tree.root_node())
        .filter(|node| matches!(node.kind(), "use_declaration" | "scoped_identifier"));
    for node in imports {
        let text = &source[node.byte_range()];
        assert!(!forbidden.is_match(text), "{} crosses the persistence boundary: {text}", file.display());
    }
}
