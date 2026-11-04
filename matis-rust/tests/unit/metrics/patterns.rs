//! Semantic regressions for lexical patterns, generic calls, and member writes.
use matis::{
    domain::models::{ParsedRepository, SourceFile},
    metrics::resolver::resolve_connections,
    parsing::parser::parse_file,
};
use std::path::PathBuf;

fn assert_graph(language: &str, source: &str) {
    let path = if language == "rust" { "patterns.rs" } else { "patterns.py" };
    let file = parse_file(SourceFile {
        path: path.into(),
        absolute_path: PathBuf::from(path),
        language: language.into(),
        source: source.as_bytes().to_vec(),
    })
    .unwrap();
    assert!(file.parse_error.is_none(), "{:?}", file.parse_error);
    let repository = ParsedRepository { files: vec![file] };
    let graph = resolve_connections(&repository).unwrap();
    for (index, line) in source.lines().enumerate() {
        let Some((_, expectation)) = line.split_once("expect:") else { continue };
        let expected = expectation.trim();
        let edges: Vec<_> =
            graph.connections.iter().filter(|edge| edge.line == index + 1 && edge.type_ == "CALLS").collect();
        assert_eq!(edges.len(), 1, "line {}: {line}; {edges:?}", index + 1);
        if expected == "external" {
            assert!(edges[0].target_key.starts_with("external:"), "{line}: {:?}", edges[0]);
        } else {
            let symbol = repository.files[0].symbols.iter().find(|symbol| symbol.name == expected).unwrap();
            assert_eq!(edges[0].target_key, symbol.key, "{line}");
        }
    }
}

#[test]
fn rust_loop_and_conditional_patterns_bind_only_their_regions() {
    assert_graph(
        "rust",
        r#"
fn target() {}
fn run() {
    target(); // expect: target
    for target in [|| {}] {
        target(); // expect: external
    }
    target(); // expect: target
    for (target, _) in [(|| {}, 0)] {
        target(); // expect: external
    }
    if let Some(target) = Some(|| {}) {
        target(); // expect: external
    } else {
        target(); // expect: target
    }
    target(); // expect: target
    while let Some(target) = Some(|| {}) {
        target(); // expect: external
    }
    target(); // expect: target
    for target in {
        target(); // expect: target
        [|| {}]
    } {
        target(); // expect: external
    }
    if let Some(target) = {
        target(); // expect: target
        Some(|| {})
    } {
        target(); // expect: external
    }
}
"#,
    );
}

#[test]
fn rust_macro_conditional_patterns_do_not_resolve_captured_calls_to_functions() {
    // Opaque payload bindings conservatively block resolution even in the else branch.
    for payload in [
        "if let Some(target) = Some(|| 2) {\n target() // expect: external\n } else { 0 }",
        "while let Some(target) = Some(|| 2) {\n target(); // expect: external\n break; }",
        "if let Some(target) = Some(|| 2) && {\n target() == 2 // expect: external\n } {\n target() // expect: external\n } else { 0 }",
        "if let Some(target) = Some(|| 2) {\n target() // expect: external\n } else {\n target() // expect: external\n }",
        "if let Some(target) = Some(|| 2) {\n println!(\"{}\", {\n target() // expect: external\n });\n 0 } else { 0 }",
    ] {
        assert_graph(
            "rust",
            &format!(
                r#"
fn target() -> i32 {{ 1 }}
fn run() {{
    target(); // expect: target
    println!("{{:?}}", {{
        {payload}
    }});
    target(); // expect: target
    println!("{{}}", {{
        target() // expect: target
    }});
}}
"#
            ),
        );
    }
}

#[test]
fn rust_macro_struct_captures_shadow_functions() {
    for payload in [
        "if let Record { target } = record {\n target() // expect: external\n } else { 0 }",
        "let Record { target } = record;\n target() // expect: external",
        "for Record { target } in [record] {\n target(); // expect: external\n }",
        "match record { Record { target } => {\n target() // expect: external\n }}",
    ] {
        assert_graph(
            "rust",
            &format!(
                r#"
fn target() -> i32 {{ 1 }}
struct Record {{ target: fn() -> i32 }}
fn run(record: Record) {{
    target(); // expect: target
    println!("{{:?}}", {{
        {payload}
    }});
    target(); // expect: target
}}
"#
            ),
        );
    }
}

#[test]
fn rust_match_patterns_cover_guards_without_blocking_other_arms() {
    assert_graph(
        "rust",
        r#"
fn target() -> bool { true }
fn Some() {}
struct Record { field: fn() }
fn run(value: Option<fn()>) {
    match value {
        Some(target) if target() => { // expect: external
            target(); // expect: external
            Some(); // expect: Some
        },
        None => {
            target(); // expect: target
        }
    }
    match (Record { field: || {} }) {
        Record { field: target } => {
            target(); // expect: external
        }
    }
    target(); // expect: target
}
"#,
    );
}

#[test]
fn rust_pattern_shorthand_and_let_chains_preserve_binding_boundaries() {
    assert_graph(
        "rust",
        r#"
fn target() -> bool { true }
struct Record { target: fn() }
fn run(record: Record) {
    match record {
        Record { target } => {
            target(); // expect: external
        }
    }
    if let Some(target) = Some(|| true) && {
        target() // expect: external
    } {
        target(); // expect: external
    } else {
        target(); // expect: target
    }
    target(); // expect: target
}
"#,
    );
}

#[test]
fn rust_declaration_and_parameter_patterns_exclude_constructor_and_field_names() {
    assert_graph(
        "rust",
        r#"
fn target() {}
fn Some() {}
fn field() {}
struct Record { field: fn() }
fn run() {
    {
        let Some(target) = Some(|| {}) else { return };
        target(); // expect: external
        Some(); // expect: Some
    }
    {
        let Record { field: target } = Record { field: || {} };
        target(); // expect: external
        field(); // expect: field
    }
    target(); // expect: target
}
fn parameter(Record { field: target }: Record) {
    target(); // expect: external
    field(); // expect: field
}
"#,
    );
}

#[test]
fn python_match_captures_bind_the_function_not_the_module() {
    assert_graph(
        "python",
        r#"
def target(): pass
def run(value):
    match value:
        case target:
            target() # expect: external
    target() # expect: external
def other():
    target() # expect: target
"#,
    );
}

#[test]
fn python_match_capture_shapes_exclude_value_patterns_and_class_names() {
    for pattern in [
        "[target, *rest]",
        "{'key': target, **rest}",
        "C(field=target)",
        "C(target)",
        "_ as target",
        "[target] | (target,)",
    ] {
        assert_graph(
            "python",
            &format!(
                r#"
def target(): pass
class C: pass
def run(value):
    match value:
        case {pattern}:
            target() # expect: external
"#
            ),
        );
    }
    for pattern in ["ns.target", "C()", "target()", "{'target': other}", "_", "C(target=other)"] {
        assert_graph(
            "python",
            &format!(
                r#"
def target(): pass
class C: pass
def run(value):
    match value:
        case {pattern}:
            target() # expect: target
"#
            ),
        );
    }
}

#[test]
fn rust_turbofish_resolves_bare_associated_and_instance_calls() {
    assert_graph(
        "rust",
        r#"
fn target<T>() {}
struct C;
impl C {
    fn hit<T>(&self) {}
    fn make<T>() {}
}
fn run() {
    target::<i32>(); // expect: target
    C::make::<i32>(); // expect: make
    let value = C {};
    value.hit::<i32>(); // expect: hit
    missing::<i32>(); // expect: external
}
fn shadow(target: fn()) {
    target::<i32>(); // expect: external
}
"#,
    );
}

#[test]
fn python_member_rebinding_ignores_all_syntax_whitespace() {
    for member in ["C . hit", "C\t.\thit", "C\t. hit"] {
        assert_graph(
            "python",
            &format!(
                r#"
class C:
    @staticmethod
    def hit(): pass
def replacement(): pass
{member} = replacement
C.hit() # expect: external
"#
            ),
        );
    }
    assert_graph(
        "python",
        r#"
class C:
    @staticmethod
    def hit(): pass
C.hitter = replacement
C.hit() # expect: hit
"#,
    );
}
