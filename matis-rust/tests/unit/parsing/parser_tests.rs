//! Test record assembly with native-shaped data, without native processing.
use super::*;
use std::path::PathBuf;
use tree_sitter_language_pack::{Span, StructureKind};

fn source(text: &str) -> SourceFile {
    SourceFile {
        path: "main.py".into(),
        absolute_path: PathBuf::from("/repo/main.py"),
        language: "python".into(),
        source: text.as_bytes().to_vec(),
    }
}

#[test]
fn native_structure_preserves_identity_metadata_coordinates_and_inheritance() -> Result<()> {
    let item = StructureItem {
        name: Some("Brew".into()),
        kind: StructureKind::Class,
        span: Span { start_byte: 0, end_byte: 30, start_line: 0, start_column: 1, end_line: 5, end_column: 6 },
        signature: Some("class Brew(Base):".into()),
        visibility: Some("public".into()),
        decorators: vec!["@coffee".into()],
        children: vec![StructureItem {
            name: Some(String::new()),
            kind: StructureKind::Method,
            span: Span { start_byte: 10, end_byte: 20, start_line: 2, start_column: 3, end_line: 4, end_column: 5 },
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut file = ParsedFile {
        source: source(""),
        content_hash: "hash".into(),
        parse_error: None,
        symbols: vec![],
        imports: vec![],
        calls: vec![],
        inheritances: vec![],
    };
    add_structure(&mut file, &item, ("file:main.py", &[]))?;
    assert_eq!(file.symbols.len(), 2);
    let outer = &file.symbols[0];
    let inner = &file.symbols[1];
    assert_eq!(outer.key, "symbol:main.py::Brew:0");
    assert_eq!(outer.parent_key, "file:main.py");
    assert_eq!(outer.signature, item.signature);
    assert_eq!(outer.visibility, item.visibility);
    assert_eq!(outer.decorators, item.decorators);
    assert_eq!((outer.start_line, outer.start_column, outer.end_line, outer.end_column), (1, 2, 6, 7));
    assert_eq!(inner.name, "<anonymous>");
    assert_eq!(inner.qualified_name, "main.py::Brew.<anonymous>");
    assert_eq!(inner.key, "symbol:main.py::Brew.<anonymous>:10");
    assert_eq!(inner.parent_key, outer.key);
    assert_eq!((inner.start_byte, inner.end_byte), (10, 20));
    assert_eq!((inner.start_line, inner.start_column, inner.end_line, inner.end_column), (3, 4, 5, 6));
    assert!(inner.complexity.is_none() && inner.line_count.is_none() && inner.parameter_count.is_none());
    assert_eq!(file.inheritances.len(), 1);
    let inheritance = &file.inheritances[0];
    assert_eq!(inheritance.source_key, outer.key);
    assert_eq!(inheritance.file_path, "main.py");
    assert_eq!(inheritance.language, "python");
    assert_eq!(inheritance.target_name, "Base");
    assert_eq!(inheritance.line, 1);
    assert_eq!(inheritance.evidence, "class Brew(Base):");
    Ok(())
}

#[test]
fn call_evidence_limits_characters_not_utf8_bytes() -> Result<()> {
    let text = format!("\nservice.brew('{}')", "é".repeat(301));
    let source = source(&text);
    let tree = syntax::parse("python", &source.source)?;
    let calls = extract_calls(&source, tree.root_node(), get_adapter("python")?);
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.name, "brew");
    assert_eq!(call.file_path, "main.py");
    assert_eq!((call.start_byte, call.end_byte, call.line), (1, text.len(), 2));
    assert_eq!(call.evidence, text[1..].chars().take(300).collect::<String>());
    assert_eq!(call.evidence.chars().count(), 300);
    assert!(!call.bare_target);
    assert_eq!(call.qualifier.as_deref(), Some("service"));
    assert_eq!(call.relationship, "CALLS");
    Ok(())
}

fn first_error_full_walk(root: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if node.is_error() || node.is_missing() {
            return Some(node);
        }
        pending.extend(children(node).into_iter().rev());
    }
    None
}

#[test]
fn syntax_diagnostic_matches_full_walk_for_nested_errors_and_missing_tokens() -> Result<()> {
    let mut saw_missing = false;
    for (language, text) in [
        ("python", "def valid():\n    helper()\n\ndef broken():\n    é = ;\nother = ;\n"),
        ("python", "def brew()\n    pass\n"),
        ("rust", "fn main() { let value = 1 }"),
        ("javascript", "function brew() { helper(); let value = ; }\nother = ;"),
    ] {
        let tree = syntax::parse(language, text.as_bytes())?;
        let first = first_error_full_walk(tree.root_node()).expect("fixture must contain an error or missing token");
        saw_missing |= first.is_missing();
        let point = first.start_position();
        assert_eq!(
            syntax_error(tree.root_node()),
            Some(format!(
                "Syntax error at line {}, column {}; analysis may be incomplete.",
                point.row + 1,
                point.column + 1
            )),
            "{language}: {text}"
        );
    }
    assert!(saw_missing, "fixtures must exercise a missing token");
    Ok(())
}

#[test]
#[ignore = "focused repeatable timing; run with --ignored --nocapture --test-threads=1"]
fn benchmark_late_syntax_error() -> Result<()> {
    let text = format!("{}broken = ;\n", "value = helper()\n".repeat(10_000));
    let tree = syntax::parse("python", text.as_bytes())?;
    let expected = "Syntax error at line 10001, column 9; analysis may be incomplete.";
    assert_eq!(syntax_error(tree.root_node()).as_deref(), Some(expected));
    let start = std::time::Instant::now();
    for _ in 0..100 {
        std::hint::black_box(syntax_error(std::hint::black_box(tree.root_node())));
    }
    eprintln!("100 diagnostics after 10,000 valid statements: {:?}", start.elapsed());
    Ok(())
}

#[test]
fn syntax_diagnostic_uses_first_error_and_one_based_byte_column() -> Result<()> {
    let text = "é = ;\nother = ;\n";
    let tree = syntax::parse("python", text.as_bytes())?;
    assert_eq!(
        syntax_error(tree.root_node()).as_deref(),
        Some("Syntax error at line 1, column 6; analysis may be incomplete.")
    );
    let valid = syntax::parse("python", b"run()")?;
    assert_eq!(syntax_error(valid.root_node()), None);
    Ok(())
}
