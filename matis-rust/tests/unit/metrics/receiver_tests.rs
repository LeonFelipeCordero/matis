use super::*;
use crate::domain::models::{Call, ParsedFile, ParsedRepository, SourceFile};
use crate::metrics::adapters::get_resolution_adapter;
use crate::metrics::receiver_syntax::{self, ReceiverBinding};
use crate::syntax::{named_children, walk};

fn parsed(language: &str, source: &str) -> ParsedFile {
    ParsedFile {
        source: SourceFile {
            path: format!("test.{language}"),
            absolute_path: format!("/test.{language}").into(),
            language: language.to_owned(),
            source: source.as_bytes().to_vec(),
        },
        content_hash: String::new(),
        parse_error: None,
        symbols: vec![],
        imports: vec![],
        calls: vec![],
        inheritances: vec![],
    }
}

fn syntax(language: &str, source: &str) -> Syntax {
    Syntax::new(&parsed(language, source), get_resolution_adapter(language))
}

fn bindings(syntax: &Syntax) -> Vec<ReceiverBinding<'_>> {
    syntax.nodes().into_iter().flat_map(|n| syntax.adapter.receiver_bindings(syntax, n)).collect()
}

fn binding<'a>(entries: &'a [ReceiverBinding<'a>], name: &str) -> &'a ReceiverBinding<'a> {
    entries.iter().find(|b| b.name == name).unwrap_or_else(|| panic!("Missing receiver {name}: {entries:?}"))
}

fn resolver(language: &str, source: &str) -> CallResolver {
    let file = crate::parsing::parser::parse_file(parsed(language, source).source).unwrap();
    CallResolver::new(&ParsedRepository { files: vec![file] })
}

fn call(resolver: &CallResolver, evidence: &str) -> Call {
    let syntax = resolver.syntax.values().next().unwrap();
    let start = String::from_utf8_lossy(&syntax.source).find(evidence).unwrap();
    let node = syntax.node(start, start + evidence.len()).unwrap_or_else(|| panic!("No call node: {evidence}"));
    Call {
        file_path: syntax.file.source.path.clone(),
        name: String::new(),
        start_byte: start,
        end_byte: node.end_byte(),
        line: node.start_position().row + 1,
        evidence: evidence.to_owned(),
        bare_target: false,
        qualifier: None,
        relationship: "CALLS".to_owned(),
    }
}

fn inferred(resolver: &CallResolver, evidence: &str, lookup: (&str, &str)) -> Option<Vec<String>> {
    let (qualifier, name) = lookup;
    let call = call(resolver, evidence);
    let request = ResolutionRequest {
        file: &resolver.files[&call.file_path],
        call: &call,
        owner: resolver.owner(&call),
        name: name.to_owned(),
        qualifier: Some(qualifier.to_owned()),
    };
    ReceiverTypes::infer(resolver, &request).map(|types| types.into_iter().map(|s| s.name).collect())
}

#[test]
fn cached_bindings_restore_exact_nodes_across_syntax_clones() {
    let original = syntax("typescript", "class Host { constructor(public dep: pkg.Thing) {} f(input: Thing) {} }");
    let map = [(original.file.source.path.clone(), original.clone())].into();
    let index = ReceiverTypes::new(&map);
    let mut cloned = original.clone();
    // A different extraction policy must not change the already cached declarations.
    cloned.adapter = get_resolution_adapter("unknown");
    let expected = bindings(&original);
    let restored = index.bindings_for(&cloned);
    assert_eq!(expected.len(), restored.len());
    for (left, right) in expected.iter().zip(&restored) {
        assert_eq!(left.name, right.name);
        assert_eq!(left.annotation, right.annotation);
        assert_eq!(left.node.kind(), right.node.kind());
        assert_eq!(left.node.byte_range(), right.node.byte_range());
        assert_eq!(left.scope.kind(), right.scope.kind());
        assert_eq!(left.scope.byte_range(), right.scope.byte_range());
        assert_eq!((left.inferred, left.member, left.static_), (right.inferred, right.member, right.static_));
    }
}

#[test]
fn named_receiver_lookup_preserves_full_index_order_and_scope_boundaries() {
    let source =
        (0..500).map(|index| format!("function f{index}(input: Thing) {{ input.run(); }}\n")).collect::<String>();
    let syntax = syntax("typescript", &source);
    let index = ReceiverTypes::new(&[(syntax.file.source.path.clone(), syntax.clone())].into());
    let all = index.bindings_for(&syntax);
    let position = source.rfind("input.run").unwrap();
    let expected: Vec<_> = all
        .iter()
        .filter(|binding| {
            binding.name == "input" && binding.scope.start_byte() <= position && position < binding.scope.end_byte()
        })
        .collect();
    let selected = index.bindings_at(&syntax, "input", position);
    assert_eq!(selected.len(), 1);
    assert_eq!(expected.len(), selected.len());
    assert_eq!(selected[0].annotation, expected[0].annotation);
    assert_eq!(selected[0].scope, expected[0].scope);
    assert_eq!(selected[0].node, expected[0].node);
    assert!(index.bindings_at(&syntax, "input", source.len()).is_empty());
    assert!(index.bindings_at(&syntax, "missing", position).is_empty());
}

#[test]
fn type_names_strip_only_supported_wrappers() {
    let syntax = syntax("rust", "fn f(a: &'a pkg::Thing<T>, b: (A, B), c: &mut Thing) {}\n");
    let parameters: Vec<_> = syntax.nodes().into_iter().filter(|n| n.kind() == "parameter").collect();
    let names: Vec<_> =
        parameters.into_iter().map(|n| receiver_syntax::type_name(field(n, &["type"]), &syntax.source)).collect();
    assert_eq!(names, vec![Some("pkg::Thing".into()), None, None]);
    assert_eq!(receiver_syntax::type_name(None, &syntax.source), None);
}

#[test]
fn type_names_match_python_unicode_word_rules() {
    for (source, expected) in [("变量", Some("变量")), ("a\u{301}", None), ("x²", Some("x²")), ("a_b", Some("a_b"))]
    {
        let syntax = syntax("python", source);
        let root = syntax.root().unwrap();
        let node = walk(root).find(|n| n.start_byte() == 0 && n.end_byte() == source.len()).unwrap();
        assert_eq!(receiver_syntax::type_name(Some(node), &syntax.source).as_deref(), expected, "{source}");
    }
}

#[test]
fn java_parameters_fields_var_and_static_flags() {
    let syntax = syntax(
        "java",
        "class Host { static Thing shared; Thing field; void f(Thing input) { var local = new Thing(); } }",
    );
    let entries = bindings(&syntax);
    assert!(binding(&entries, "shared").static_);
    assert!(binding(&entries, "field").member);
    assert!(!binding(&entries, "input").member);
    let local = binding(&entries, "local");
    assert!(local.inferred);
    assert_eq!(local.annotation.as_deref(), Some("Thing"));
}

#[test]
fn kotlin_class_parameters_and_properties() {
    let syntax = syntax(
        "kotlin",
        "class Host(val field: Thing, plain: Thing) { val local: Thing = Thing()\n fun f(input: Thing) {} }",
    );
    let entries = bindings(&syntax);
    assert!(binding(&entries, "field").member);
    assert!(binding(&entries, "local").member);
    assert_eq!(binding(&entries, "input").annotation.as_deref(), Some("Thing"));
    assert!(!entries.iter().any(|b| b.name == "plain"));
}

#[test]
fn python_members_and_constructor_injection() {
    let syntax = syntax(
        "python",
        "class Host:\n field: Thing\n def __init__(self, input: Thing):\n  self.dep = input\n def f(self):\n  self.late = Thing()\n",
    );
    let entries = bindings(&syntax);
    assert!(binding(&entries, "field").member);
    let dep = binding(&entries, "dep");
    assert_eq!(dep.annotation.as_deref(), Some("Thing"));
    assert!(dep.inferred && dep.member);
    assert!(binding(&entries, "late").annotation.is_none());
}

#[test]
fn injection_rejects_reassigned_parameters() {
    let syntax = syntax("typescript", "class Host { constructor(input: Thing) { input = other; this.dep = input; } }");
    let entries = bindings(&syntax);
    assert_eq!(binding(&entries, "dep").annotation, None);
}

#[test]
fn script_parameter_properties_and_inference() {
    let syntax = syntax(
        "typescript",
        "class Host { static shared: Thing; constructor(public dep: Thing) {} f(input: Thing) { let local = new Thing(); } }",
    );
    let entries = bindings(&syntax);
    assert!(binding(&entries, "shared").static_);
    assert!(binding(&entries, "dep").member);
    assert_eq!(binding(&entries, "input").annotation.as_deref(), Some("Thing"));
    assert_eq!(binding(&entries, "local").annotation.as_deref(), Some("Thing"));
}

#[test]
fn go_short_declarations_pair_only_one_value_per_name() {
    let syntax = syntax("go", "package p\nfunc f(a, b Thing) { x, y := Thing{}, Other{}; u, v := pair() }\n");
    let entries = bindings(&syntax);
    for name in ["a", "b", "x"] {
        assert_eq!(binding(&entries, name).annotation.as_deref(), Some("Thing"));
    }
    assert_eq!(binding(&entries, "y").annotation.as_deref(), Some("Other"));
    for name in ["u", "v"] {
        assert!(binding(&entries, name).annotation.is_none());
    }
}

#[test]
fn rust_ignores_destructured_let_and_infers_struct_literals() {
    let syntax = syntax(
        "rust",
        "struct Host { field: Thing } fn f(input: Thing) { let local = Thing {}; let (x, y) = pair(); }",
    );
    let entries = bindings(&syntax);
    assert!(binding(&entries, "field").member);
    assert_eq!(binding(&entries, "local").annotation.as_deref(), Some("Thing"));
    assert_eq!(binding(&entries, "input").annotation.as_deref(), Some("Thing"));
    assert!(!entries.iter().any(|b| b.name == "x" || b.name == "y"));
}

#[test]
fn receiver_inference_distinguishes_absent_and_unsafe_types() {
    let resolver = resolver(
        "python",
        "class Thing:\n def run(self): pass\ndef f(x: Thing):\n x.run()\n unknown.run()\n missing = factory()\n missing.run()\n",
    );
    assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec!["Thing".into()]));
    assert_eq!(inferred(&resolver, "unknown.run()", ("unknown", "run")), None);
    assert_eq!(inferred(&resolver, "missing.run()", ("missing", "run")), Some(vec![]));
    let call = call(&resolver, "x.run()");
    let request = ResolutionRequest {
        file: &resolver.files[&call.file_path],
        call: &call,
        owner: resolver.owner(&call),
        name: "run".into(),
        qualifier: Some("x".into()),
    };
    let members = ReceiverTypes::resolve(&resolver, &request).unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].name, "run");
}

#[test]
fn inference_rejects_conditional_and_later_bindings() {
    for body in [" if flag:\n  x = Thing()\n x.run()", " x.run()\n x = Thing()", " if flag:\n  x: Thing\n x.run()"] {
        let resolver = resolver("python", &format!("class Thing:\n def run(self): pass\ndef f():\n{body}\n"));
        assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec![]), "{body}");
    }
}

#[test]
fn explicit_annotations_must_match_known_initializers() {
    for (value, expected) in
        [("Thing()", vec!["Thing".to_owned()]), ("Other()", vec![]), ("factory()", vec![]), ("None", vec![])]
    {
        let resolver = resolver(
            "python",
            &format!(
                "class Thing:\n def run(self): pass\nclass Other: pass\ndef f():\n x: Thing = {value}\n x.run()\n"
            ),
        );
        assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(expected), "{value}");
    }
}

#[test]
fn nearest_binding_prefers_declared_over_inferred() {
    let resolver = resolver("python", "class Thing:\n def run(self): pass\ndef f(x: Thing):\n x = Thing()\n x.run()\n");
    let inference = ReceiverInference::new(&resolver);
    let bindings = inference.bindings(resolver.files.keys().next().unwrap());
    let entries: Vec<_> = bindings.iter().filter(|b| b.name == "x").collect();
    let nearest = ReceiverInference::nearest(entries);
    assert_eq!(nearest.len(), 1);
    assert!(!nearest[0].inferred);
    assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec![]));
}

fn path_written_for(resolver: &CallResolver, evidence: &str, receiver: bool) -> bool {
    let call = call(resolver, evidence);
    let request = ResolutionRequest {
        file: &resolver.files[&call.file_path],
        call: &call,
        owner: resolver.owner(&call),
        name: "run".into(),
        qualifier: Some("x".into()),
    };
    ReceiverInference::new(resolver).path_written(&request, receiver)
}

#[test]
fn expanding_alias_cycles_check_observed_member_writes() {
    let cases = [
        ("x = x.field", "", false),
        ("x = x.field", "x.field.run = other", true),
        ("x = x.field", "x.b.a.run = other", false),
        ("x = y.a\n y = x.b", "", false),
        ("x = y.a\n y = x.b", "x.field.run = other", false),
        ("x = y.a\n y = x.b", "x.b.a.run = other", true),
        ("x = x.a\n x = x.b", "", false),
        ("x = x.a\n x = x.b", "x.field.run = other", false),
        ("x = x.a\n x = x.b", "x.b.a.run = other", true),
    ];
    for (aliases, write, expected) in cases {
        let resolver = resolver(
            "python",
            &format!("class Thing:\n def run(self): pass\ndef f(x: Thing):\n {aliases}\n {write}\n x.run()\n"),
        );
        assert_eq!(path_written_for(&resolver, "x.run()", false), expected, "{aliases}; {write}");
        assert!(path_written_for(&resolver, "x.run()", true));
        assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec![]));
    }
}

#[test]
fn unrelated_expanding_cycles_do_not_block_safe_receivers() {
    let resolver = resolver(
        "python",
        "class Thing:\n def run(self): pass\ndef f(x: Thing):\n y = y.field\n y = z.a\n z = y.b\n x.run()\n",
    );
    assert!(!path_written_for(&resolver, "x.run()", false));
    assert!(!path_written_for(&resolver, "x.run()", true));
    assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec!["Thing".into()]));
}

#[test]
fn replacing_an_alias_of_a_member_does_not_write_that_member() {
    let resolver = resolver(
        "python",
        "class Thing:\n def run(self): pass\ndef f(x: Thing):\n alias = x.run\n alias = other\n x.run()\n",
    );
    assert!(!path_written_for(&resolver, "x.run()", false));
    assert!(!path_written_for(&resolver, "x.run()", true));
    assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec!["Thing".into()]));
}

#[test]
fn receiver_and_alias_member_writes_reject_identity() {
    for statement in ["x = Thing()", "x.run = other", "alias = x\n alias.run = other", "Thing.run = other"] {
        let resolver = resolver(
            "python",
            &format!("class Thing:\n def run(self): pass\ndef f(x: Thing):\n x.run()\n {statement}\n"),
        );
        assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec![]), "{statement}");
    }
}

#[test]
fn field_chain_uses_unique_declared_member_type() {
    let resolver = resolver(
        "python",
        "class Thing:\n def run(self): pass\nclass Host:\n dep: Thing\ndef f(host: Host):\n host.dep.run()\n",
    );
    assert_eq!(inferred(&resolver, "host.dep.run()", ("host.dep", "run")), Some(vec!["Thing".into()]));
}

#[test]
fn java_trusted_field_constructor_injection_is_not_reassignment() {
    let resolver = resolver(
        "java",
        "class Thing { void run() {} } class Host { Thing dep; Host(Thing input) { this.dep = input; } void f() { dep.run(); } }",
    );
    assert_eq!(inferred(&resolver, "dep.run()", ("dep", "run")), Some(vec!["Thing".into()]));
}

#[test]
fn script_identity_write_through_prototype_rejects_instance_type() {
    let resolver = resolver(
        "typescript",
        "class Thing { run() {} } function f() { const x = new Thing(); x.run(); Thing.prototype.run = other; }",
    );
    assert_eq!(inferred(&resolver, "x.run()", ("x", "run")), Some(vec![]));
}

#[test]
fn member_write_names_include_full_path_but_not_nested_member_names() {
    let syntax = syntax("python", "def f():\n a.b = value\n x, y = pair()\n");
    let assignments: Vec<_> = syntax.nodes().into_iter().filter(|n| n.kind() == "assignment").collect();
    assert_eq!(ReceiverInference::write_names(&syntax, assignments[0]), ["a.b".to_owned()].into());
    let writes = ReceiverInference::write_names(&syntax, assignments[1]);
    assert!(writes.contains("x") && writes.contains("y") && writes.contains("x,y"));
    assert_eq!(named_children(assignments[0])[0].kind(), "attribute");
}
