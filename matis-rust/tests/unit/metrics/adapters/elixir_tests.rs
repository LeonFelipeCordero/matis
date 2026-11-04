use super::*;
use crate::{
    domain::models::{Call, SourceFile},
    metrics::adapters::adapter_for_language,
    parsing::parser::parse_file,
    syntax::walk,
};
use std::path::PathBuf;
fn parser() -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language("elixir").unwrap()).unwrap();
    parser
}

fn file(source: &str) -> ParsedFile {
    parse_file(SourceFile {
        path: "a.ex".into(),
        absolute_path: PathBuf::from("/matis-module-tests/a.ex"),
        language: "elixir".into(),
        source: source.as_bytes().to_vec(),
    })
    .unwrap()
}
fn select_call(file: &ParsedFile, evidence: &str, matches: Vec<Symbol>) -> Vec<Symbol> {
    let syntax = BTreeMap::from([(file.source.path.clone(), Syntax::new(file, adapter_for_language("elixir")))]);
    let repository = ParsedRepository { files: vec![file.clone()] };
    let identity = CallableIdentity { clauses: clauses(&repository, &syntax), syntax };
    let start = String::from_utf8_lossy(&file.source.source).rfind(evidence).unwrap();
    let call = Call {
        file_path: file.source.path.clone(),
        name: "run".into(),
        start_byte: start,
        end_byte: start + evidence.len(),
        line: 1,
        evidence: evidence.into(),
        bare_target: true,
        qualifier: evidence.contains('.').then(|| "A".into()),
        relationship: "CALLS".into(),
    };
    let request =
        ResolutionRequest { file, call: &call, owner: None, name: "run".into(), qualifier: call.qualifier.clone() };
    select(&identity, &request, matches)
}
#[test]
fn clauses_group_by_parent_name_and_filter_arity_defaults_and_privacy() {
    let file = file(
        "defmodule A do\n def run(x, y \\\\ 1), do: x+y\n def run(:ok, y), do: y\n defp hidden(x), do: x\n run(1)\n run(1, 2)\n A.hidden(1)\nend\n",
    );
    let run: Vec<_> = file.symbols.iter().filter(|s| s.name == "run").cloned().collect();
    assert_eq!(run.len(), 2);
    assert_eq!(select_call(&file, "run(1)", run.clone()).len(), 1);
    assert_eq!(select_call(&file, "run(1, 2)", run).len(), 2);
    let hidden: Vec<_> = file.symbols.iter().filter(|s| s.name == "hidden").cloned().collect();
    assert!(select_call(&file, "A.hidden(1)", hidden).is_empty());
}
#[test]
fn same_name_in_different_modules_does_not_form_one_clause_identity() {
    let file = file("defmodule A do\n def run(x), do: x\nend\ndefmodule B do\n def run(x), do: x\n run(1)\nend\n");
    let run = file.symbols.iter().filter(|s| s.name == "run").cloned().collect();
    assert!(select_call(&file, "run(1)", run).is_empty());
}
#[test]
fn pipelines_add_one_argument_including_bare_identifiers() {
    let source = b"x |> run(1)\nx |> run\nrun()\n";
    let tree = parser().parse(source, None).unwrap();
    let nodes: Vec<_> = walk(tree.root_node()).collect();
    let call = nodes.iter().find(|n| text(Some(**n), source) == "run(1)").unwrap();
    let identifier = nodes
        .iter()
        .find(|n| {
            n.kind() == "identifier"
                && text(Some(**n), source) == "run"
                && n.parent().is_some_and(|p| p.kind() == "binary_operator")
        })
        .unwrap();
    assert_eq!(call_arity(Some(*call), source), Some(2));
    assert_eq!(call_arity(Some(*identifier), source), Some(1));
    assert_eq!(arity(Some(*identifier)), None);
    assert_eq!(call_arity(None, source), None);
}
#[test]
fn decision_rules_exclude_fallbacks_and_anonymous_clauses() {
    let source = b"case x do\n _ -> 0\n _ when true -> 1\nend\ncond do\n x -> 1\n true -> 0\nend\nfn x -> x end\n";
    let tree = parser().parse(source, None).unwrap();
    let decisions: Vec<_> =
        walk(tree.root_node()).filter(|n| n.kind() == "stab_clause").map(|n| clause_decision(n, source)).collect();
    assert_eq!(decisions, [false, true, true, false, false]);
}
#[test]
fn parameter_counts_support_guards_zero_inputs_and_anonymous_clause_consistency() {
    let source = b"def zero, do: 0\ndef f(x, y) when x > 0, do: y\nfn x -> x; y -> y end\nfn x -> x; x,y -> y end\n";
    let tree = parser().parse(source, None).unwrap();
    let counts: Vec<_> = named_children(tree.root_node()).into_iter().map(|n| parameter_count(n, source)).collect();
    assert_eq!(counts, [Some(0), Some(2), Some(1), None]);
}
#[test]
fn import_paths_use_python_unicode_words() {
    assert_eq!(import_path("\u{301}alias αⅧ.X\u{301}tail"), "αⅧ.X");
}
#[test]
fn only_alias_and_import_calls_are_import_nodes() {
    let source = b"alias A.B\nimport C\nrequire D\nuse E\n";
    let tree = parser().parse(source, None).unwrap();
    let actual: Vec<_> = named_children(tree.root_node()).into_iter().map(|n| is_import_node(n, source)).collect();
    assert_eq!(actual, [true, true, false, false]);
}
