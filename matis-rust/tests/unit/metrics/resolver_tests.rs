use super::*;
use crate::domain::models::{Import, Inheritance, SourceFile};
use std::path::PathBuf;

fn symbol(identity: (&str, &str, &str), declaration: (&str, usize, usize)) -> Symbol {
    let (key, name, parent) = identity;
    let (kind, start, end) = declaration;
    Symbol {
        key: key.into(),
        name: name.into(),
        parent_key: parent.into(),
        kind: kind.into(),
        file_path: "a.py".into(),
        language: "python".into(),
        qualified_name: name.into(),
        start_byte: start,
        end_byte: end,
        start_line: 1,
        end_line: 1,
        start_column: 0,
        end_column: 0,
        signature: None,
        visibility: None,
        decorators: vec![],
        complexity: None,
        line_count: None,
        parameter_count: None,
    }
}
fn file(source: &str, symbols: Vec<Symbol>) -> ParsedFile {
    ParsedFile {
        source: SourceFile {
            path: "a.py".into(),
            absolute_path: PathBuf::from("a.py"),
            language: "python".into(),
            source: source.as_bytes().to_vec(),
        },
        content_hash: String::new(),
        parse_error: None,
        symbols,
        imports: vec![],
        calls: vec![],
        inheritances: vec![],
    }
}
fn call(start: usize, end: usize, name: &str) -> Call {
    Call {
        file_path: "a.py".into(),
        name: name.into(),
        start_byte: start,
        end_byte: end,
        line: 1,
        evidence: format!("{name}()"),
        bare_target: true,
        qualifier: None,
        relationship: "CALLS".into(),
    }
}
fn resolver(file: ParsedFile) -> CallResolver {
    CallResolver::new(&ParsedRepository { files: vec![file] })
}

fn full_walk_import<'a>(syntax: &'a Syntax, line: usize, query: Option<&str>) -> Option<Node<'a>> {
    let mut candidates: Vec<_> = syntax
        .nodes()
        .into_iter()
        .filter(|node| {
            node.start_position().row + 1 == line && syntax.adapter.is_import_node_with_source(*node, &syntax.source)
        })
        .collect();
    if let Some(query) = query {
        let exact: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|node| syntax.text(*node).trim().trim_end_matches(';') == query.trim().trim_end_matches(';'))
            .collect();
        if !exact.is_empty() {
            candidates = exact;
        }
    }
    if candidates.len() == 1 { Some(candidates[0]) } else { None }
}

fn assert_import_lookup_matches_walk(syntax: &Syntax, language: &str, line: usize) {
    let candidates: Vec<_> = syntax
        .nodes()
        .into_iter()
        .filter(|node| {
            node.start_position().row + 1 == line && syntax.adapter.is_import_node_with_source(*node, &syntax.source)
        })
        .collect();
    let spellings: Vec<_> = candidates.iter().map(|node| syntax.text(*node)).collect();
    for query in [None, Some("unknown import")].into_iter().chain(spellings.iter().map(|s| Some(s.as_str()))) {
        let expected = full_walk_import(syntax, line, query);
        assert_eq!(syntax.import_node(line, query), expected, "{language}:{line}:{query:?}");
    }
}

#[test]
fn indexed_import_lookup_matches_full_walk_including_ambiguity_and_missing_lines() {
    for (language, source) in [
        ("python", "import first; import second\ndef f():\n import nested\n"),
        ("javascript", "import { x } from 'first';\nconst y = require('second');\n"),
        ("rust", "use first::{x, y};\nmod nested { use second::z; }\n"),
        ("elixir", "alias First\nimport Second\nrequire Third\n"),
    ] {
        let mut parsed = file(source, vec![]);
        parsed.source.language = language.into();
        let syntax = Syntax::new(&parsed, get_resolution_adapter(language));
        (0..=source.lines().count() + 1).for_each(|line| assert_import_lookup_matches_walk(&syntax, language, line));
    }
}

#[test]
fn enclosing_owner_includes_exact_boundaries_and_uses_narrowest_span() {
    let resolver = resolver(file(
        "",
        vec![
            symbol(("outer", "outer", "file:a.py"), ("function", 0, 100)),
            symbol(("inner", "inner", "outer"), ("function", 20, 25)),
            symbol(("overlap", "overlap", "outer"), ("function", 21, 30)),
        ],
    ));
    assert_eq!(resolver.owner(&call(20, 25, "f")).unwrap().key, "inner");
    assert!(resolver.owner(&call(101, 102, "f")).is_none());
}
#[test]
fn ancestors_stop_on_cycle_and_lexical_ignores_unrelated_nested_symbols() {
    let owner = symbol(("owner", "owner", "cycle"), ("function", 0, 10));
    let cycle = symbol(("cycle", "cycle", "owner"), ("function", 0, 10));
    let resolver = resolver(file(
        "",
        vec![
            owner.clone(),
            cycle,
            symbol(("unrelated", "run", "elsewhere"), ("function", 0, 0)),
            symbol(("global", "run", "file:a.py"), ("function", 0, 0)),
            symbol(("local", "run", "owner"), ("function", 0, 0)),
        ],
    ));
    assert_eq!(resolver.ancestors(Some(&owner)).len(), 2);
    assert_eq!(resolver.lexical(&resolver.files["a.py"], "run", Some(&owner))[0].key, "local");
    assert_eq!(resolver.lexical(&resolver.files["a.py"], "run", None)[0].key, "global");
}
#[test]
fn named_lookup_does_not_guess_unrelated_declarations_without_fallback() {
    let resolver = resolver(file(
        "",
        vec![
            symbol(("hidden", "run", "elsewhere"), ("function", 0, 0)),
            symbol(("type", "run", "elsewhere"), ("class", 0, 0)),
        ],
    ));
    let call = call(0, 0, "run");
    let request = ResolutionRequest {
        file: &resolver.files["a.py"],
        call: &call,
        owner: None,
        name: "run".into(),
        qualifier: None,
    };
    assert!(resolver.named(&request, false).is_empty());
    assert_eq!(resolver.named(&request, true).iter().map(|s| s.key.as_str()).collect::<Vec<_>>(), ["hidden"]);
}
#[test]
fn visible_assignment_blocks_global_fallback() {
    let source = "run = replacement\nrun()\n";
    let resolver = resolver(file(source, vec![symbol(("hidden", "run", "elsewhere"), ("function", 0, 0))]));
    let start = source.rfind("run()").unwrap();
    let site = call(start, start + 5, "run");
    assert!(resolver.resolve(&site, None).is_empty());
}
#[test]
fn public_edges_keep_order_evidence_relationship_and_external_identity() {
    let source = "import missing\nclass Derived(Base): pass\nunknown()\nunknown()\n";
    let start = source.find("class").unwrap();
    let end = source.find("\nunknown").unwrap();
    let mut file = file(source, vec![symbol(("derived", "Derived", "file:a.py"), ("class", start, end))]);
    file.imports.push(Import { file_path: "a.py".into(), source: "import missing".into(), line: 1 });
    file.inheritances.push(Inheritance {
        source_key: "derived".into(),
        file_path: "a.py".into(),
        language: "python".into(),
        target_name: "Base".into(),
        line: 2,
        evidence: "class Derived(Base)".into(),
    });
    let start = source.find("unknown()").unwrap();
    let mut reference = call(start, start + 9, "unknown");
    reference.relationship = "REFERENCES".into();
    let later = source.rfind("unknown()").unwrap();
    file.calls = vec![reference, call(later, later + 9, "unknown")];
    let graph = resolve_connections(&ParsedRepository { files: vec![file] }).unwrap();
    assert_eq!(
        graph.connections.iter().map(|e| e.type_.as_str()).collect::<Vec<_>>(),
        ["IMPORTS", "INHERITS", "REFERENCES", "CALLS"]
    );
    assert_eq!(
        graph.external_nodes.iter().map(|n| n.kind.as_str()).collect::<Vec<_>>(),
        ["module", "type", "callable"]
    );
    assert_eq!(graph.connections[2].target_key, graph.connections[3].target_key);
    assert_eq!(graph.connections[2].source_key, "file:a.py");
    assert_eq!(graph.connections[0].confidence, "EXTRACTED");
    assert_eq!(graph.connections[0].evidence.as_deref(), Some("import missing"));
}
#[test]
fn resolved_call_uses_containing_symbol_as_source() {
    let source = "def target(): pass\ndef caller():\n target()\n";
    let caller = source.find("def caller").unwrap();
    let target = symbol(("target", "target", "file:a.py"), ("function", 0, source.find('\n').unwrap()));
    let mut file =
        file(source, vec![target, symbol(("caller", "caller", "file:a.py"), ("function", caller, source.len() - 1))]);
    let start = source.rfind("target()").unwrap();
    file.calls.push(call(start, start + 8, "target"));
    let graph = resolve_connections(&ParsedRepository { files: vec![file] }).unwrap();
    assert_eq!(graph.connections[0].source_key, "caller");
    assert_eq!(graph.connections[0].target_key, "target");
    assert!(graph.external_nodes.is_empty());
}
