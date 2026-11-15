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

fn assert_calls(language: &str, text: &str, expected: &[(&str, &str, Option<&str>, bool, &str)]) -> Result<()> {
    let source = SourceFile { language: language.to_owned(), ..source(text) };
    let tree = syntax::parse(language, &source.source)?;
    let calls = extract_calls(&source, tree.root_node(), get_adapter(language)?);
    let actual: Vec<_> = calls
        .iter()
        .map(|call| {
            (
                call.evidence.as_str(),
                call.name.as_str(),
                call.qualifier.as_deref(),
                call.bare_target,
                call.relationship.as_str(),
            )
        })
        .collect();
    assert_eq!(actual, expected, "{language}: {text}");
    Ok(())
}

#[test]
fn standard_call_selection_preserves_nested_calls_constructors_and_references() -> Result<()> {
    for (language, text, expected) in [
        (
            "python",
            "def run():\n    service.brew(inner())\n    Factory()\n",
            vec![
                ("service.brew(inner())", "brew", Some("service"), false, "CALLS"),
                ("inner()", "inner", None, true, "CALLS"),
                ("Factory()", "Factory", None, true, "CALLS"),
            ],
        ),
        (
            "go",
            "package coffee\nfunc run() { service.Brew(inner()); Factory() }",
            vec![
                ("service.Brew(inner())", "Brew", Some("service"), false, "CALLS"),
                ("inner()", "inner", None, true, "CALLS"),
                ("Factory()", "Factory", None, true, "CALLS"),
            ],
        ),
        (
            "java",
            "class Coffee { void run() { service.brew(inner()); new Cup(); Object a = Coffee::brew; Object b = Cup::new; } }",
            vec![
                ("service.brew(inner())", "brew", Some("service"), false, "CALLS"),
                ("inner()", "inner", None, true, "CALLS"),
                ("new Cup()", "Cup", None, true, "CALLS"),
                ("Coffee::brew", "brew", Some("Coffee"), false, "REFERENCES"),
                ("Cup::new", "new", Some("Cup"), false, "REFERENCES"),
            ],
        ),
        (
            "kotlin",
            "fun run() { service.brew(inner()); Cup(); val a = Coffee::brew; val b = ::helper }",
            vec![
                ("service.brew(inner())", "brew", Some("service"), false, "CALLS"),
                ("inner()", "inner", None, true, "CALLS"),
                ("Cup()", "Cup", None, true, "CALLS"),
                ("Coffee::brew", "brew", Some("Coffee"), false, "REFERENCES"),
                ("::helper", "helper", None, true, "REFERENCES"),
            ],
        ),
    ] {
        assert_calls(language, text, &expected)?;
    }
    Ok(())
}

#[test]
fn script_call_selection_keeps_require_members_constructors_and_nested_order() -> Result<()> {
    for language in ["javascript", "typescript", "tsx"] {
        assert_calls(
            language,
            "const dep = require('coffee'); service.brew(inner()); new cups.Cup(); factory()();",
            &[
                ("require('coffee')", "require", None, true, "CALLS"),
                ("service.brew(inner())", "brew", Some("service"), false, "CALLS"),
                ("inner()", "inner", None, true, "CALLS"),
                ("new cups.Cup()", "Cup", Some("cups"), false, "CALLS"),
                ("factory()()", "factory()", None, false, "CALLS"),
                ("factory()", "factory", None, true, "CALLS"),
            ],
        )?;
    }
    for language in ["javascript", "tsx"] {
        assert_calls(
            language,
            "const view = <><Card><Child /></Card><Éclair /><éclair /><_Private /><$Widget /><cards.lower /><ui.cards.Cup /><div /><my-widget /><Upper-tag /></>;",
            &[
                ("Card", "Card", None, true, "REFERENCES"),
                ("Child", "Child", None, true, "REFERENCES"),
                ("Éclair", "Éclair", None, true, "REFERENCES"),
                ("éclair", "éclair", None, true, "REFERENCES"),
                ("_Private", "_Private", None, true, "REFERENCES"),
                ("$Widget", "$Widget", None, true, "REFERENCES"),
                ("cards.lower", "lower", Some("cards"), false, "REFERENCES"),
                ("ui.cards.Cup", "Cup", Some("ui.cards"), false, "REFERENCES"),
            ],
        )?;
    }
    Ok(())
}

#[test]
fn elixir_call_selection_keeps_guard_and_body_calls_but_not_declaration_heads() -> Result<()> {
    assert_calls(
        "elixir",
        r#"defmodule Coffee do
  alias Coffee.Tools
  import Tools
  use Plug
  require Logger
  def brew(x \\ default_value()) when is_binary(x) do
    if ready(), do: serve()
    unless done(), do: wait()
    case input() do
      _ -> inspect()
    end
    cond do
      allowed() -> choose()
    end
    with y <- fetch(), do: finish(y)
    for y <- items(), do: map(y)
    receive do
      _ -> handle()
    end
    try do
      attempt()
    after
      cleanup()
    end
    x |> prepare |> Tools.remote |> pour() |> Tools.finish()
    x |> (not_a_call)
    x |> :not_a_call
  end
  defp private_head(), do: private_body()
  defmacro macro_head(x), do: x
  defmacrop private_macro_head(x), do: x
  defguard guard_head(x) when is_atom(x)
  defguardp private_guard_head(x) when is_list(x)
  defdelegate delegated_head(x), to: Tools
end
 defprotocol Brewable do
   def protocol_head(x)
 end
 defimpl Brewable, for: Cup do
   def protocol_head(x), do: implemented(x)
 end"#,
        &[
            ("is_binary(x)", "is_binary", None, true, "CALLS"),
            ("ready()", "ready", None, true, "CALLS"),
            ("serve()", "serve", None, true, "CALLS"),
            ("done()", "done", None, true, "CALLS"),
            ("wait()", "wait", None, true, "CALLS"),
            ("input()", "input", None, true, "CALLS"),
            ("inspect()", "inspect", None, true, "CALLS"),
            ("allowed()", "allowed", None, true, "CALLS"),
            ("choose()", "choose", None, true, "CALLS"),
            ("fetch()", "fetch", None, true, "CALLS"),
            ("finish(y)", "finish", None, true, "CALLS"),
            ("items()", "items", None, true, "CALLS"),
            ("map(y)", "map", None, true, "CALLS"),
            ("handle()", "handle", None, true, "CALLS"),
            ("attempt()", "attempt", None, true, "CALLS"),
            ("cleanup()", "cleanup", None, true, "CALLS"),
            ("prepare", "prepare", None, true, "CALLS"),
            ("Tools.remote", "remote", Some("Tools"), false, "CALLS"),
            ("pour()", "pour", None, true, "CALLS"),
            ("Tools.finish()", "finish", Some("Tools"), false, "CALLS"),
            ("private_body()", "private_body", None, true, "CALLS"),
            ("is_atom(x)", "is_atom", None, true, "CALLS"),
            ("is_list(x)", "is_list", None, true, "CALLS"),
            ("implemented(x)", "implemented", None, true, "CALLS"),
        ],
    )
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
