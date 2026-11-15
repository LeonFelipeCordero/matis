use crate::{domain::models::SourceFile, parsing::parser::parse_file};
use anyhow::Result;
use std::path::PathBuf;

fn parse(language: &str, source: &str) -> Result<crate::domain::models::ParsedFile> {
    parse_file(SourceFile {
        path: "sample".to_owned(),
        absolute_path: PathBuf::from("sample"),
        language: language.to_owned(),
        source: source.as_bytes().to_vec(),
    })
}

#[test]
fn java_constructors_references_and_native_headers() -> Result<()> {
    let file = parse(
        "java",
        "package coffee; class Child extends Base implements Reader { public Child() {} void run() { Runnable f = this::run; new Child(); } }",
    )?;
    let constructor = file.symbols.iter().find(|s| s.kind == "constructor").unwrap();
    assert_eq!(constructor.name, "Child");
    assert_eq!(constructor.signature.as_deref(), Some("public Child() {}"));
    assert_eq!(constructor.visibility.as_deref(), Some("public"));
    assert_eq!(file.inheritances.iter().map(|i| i.target_name.as_str()).collect::<Vec<_>>(), ["Base", "Reader"]);
    assert!(file.calls.iter().any(|call| call.name == "run" && call.relationship == "REFERENCES"));
    assert!(file.symbols.iter().any(|symbol| symbol.name == "coffee" && symbol.kind == "module"));
    Ok(())
}

#[test]
fn typescript_keeps_only_callable_interface_properties() -> Result<()> {
    let file = parse(
        "typescript",
        "interface Reader { name: string; read: (n: number) => string; nested: ((n: number) => string); } const handler = (() => work());",
    )?;
    let names: Vec<_> = file.symbols.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(names.contains(&"read"));
    assert!(names.contains(&"nested"));
    assert!(names.contains(&"handler"));
    assert!(!names.contains(&"name"));
    Ok(())
}

#[test]
fn kotlin_companions_interfaces_imports_and_base_types() -> Result<()> {
    let file = parse(
        "kotlin",
        "package coffee\nimport coffee.Base\ninterface Reader { fun read(): Int }\nclass Child : Base(), Reader { companion object {} fun read() = 1 }",
    )?;
    assert!(file.symbols.iter().any(|symbol| symbol.name == "Companion"));
    assert!(file.symbols.iter().any(|symbol| symbol.name == "Reader" && symbol.kind == "interface"));
    assert!(file.symbols.iter().any(|symbol| symbol.name == "read"));
    assert!(file.imports.iter().any(|import| import.source == "import coffee.Base"));
    assert_eq!(file.inheritances.iter().map(|i| i.target_name.as_str()).collect::<Vec<_>>(), ["Base", "Reader"]);
    Ok(())
}

#[test]
fn go_literals_use_parallel_assignment_positions() -> Result<()> {
    let file = parse(
        "go",
        "package coffee\nfunc run() { first, second := func(){ work() }, func(){ brew() }; first(); second() }",
    )?;
    assert!(file.symbols.iter().any(|symbol| symbol.name == "first" && symbol.kind == "function"));
    assert!(file.symbols.iter().any(|symbol| symbol.name == "second" && symbol.kind == "function"));
    assert_eq!(
        file.calls.iter().map(|call| call.name.as_str()).collect::<Vec<_>>(),
        ["work", "brew", "first", "second"]
    );
    Ok(())
}

#[test]
fn elixir_form_heads_are_not_calls_and_macro_kind_keeps_wire_text() -> Result<()> {
    let file = parse(
        "elixir",
        "defmodule Coffee do\n def brew(x) when is_binary(x), do: x |> prepare |> serve()\n defmacro wrap(x), do: x\nend",
    )?;
    assert_eq!(file.calls.iter().map(|call| call.name.as_str()).collect::<Vec<_>>(), ["is_binary", "prepare", "serve"]);
    assert_eq!(file.symbols.iter().find(|symbol| symbol.name == "wrap").unwrap().kind, "{\"other\":\"macro\"}");
    Ok(())
}

#[test]
fn rust_external_modules_and_opaque_shadowed_targets() -> Result<()> {
    let file = parse(
        "rust",
        "mod child; mod inline { fn inner() {} } fn main() { println!(\"{}\", { let helper = || 1; helper() }); std::println!(\"{}\", nested!()); }",
    )?;
    assert!(file.imports.iter().any(|import| import.source == "mod child;"));
    assert!(!file.imports.iter().any(|import| import.source.starts_with("mod inline")));
    assert!(!file.calls.iter().find(|call| call.name == "helper").unwrap().bare_target);
    assert!(file.calls.iter().any(|call| call.name == "nested!"));
    Ok(())
}

#[test]
fn python_decorators_keep_exact_text_names_and_definition_boundaries() -> Result<()> {
    let file = parse(
        "python",
        "@trace(\"café\")\n@pkg.decorator(\n    flag=True,\n)\nclass Outer(pkg.Base[T], Other, metaclass=Meta):\n    @staticmethod\n    @audit\n    def run():\n        @inner\n        def nested(): pass\n",
    )?;
    let outer = file.symbols.iter().find(|symbol| symbol.name == "Outer").unwrap();
    assert_eq!(outer.decorators, ["@trace(\"café\")", "@pkg.decorator(\n    flag=True,\n)"]);
    let run = file.symbols.iter().find(|symbol| symbol.name == "run").unwrap();
    assert_eq!(run.decorators, ["@staticmethod", "@audit"]);
    let nested = file.symbols.iter().find(|symbol| symbol.name == "nested").unwrap();
    assert_eq!(nested.decorators, ["@inner"]);
    assert_eq!(
        file.inheritances.iter().map(|base| base.target_name.as_str()).collect::<Vec<_>>(),
        ["pkg.Base", "Other"]
    );
    Ok(())
}

#[test]
fn modifier_metadata_does_not_include_nested_declarations() -> Result<()> {
    use crate::parsing::{
        adapters::get_adapter,
        queries,
        syntax::{modifier_metadata, parse},
    };
    for (language, source, visibility, annotation) in [
        (
            "java",
            "@Trace(value = 1) public class Outer { @Inner private void hidden() {} }",
            "public",
            "@Trace(value = 1)",
        ),
        ("java", "@Trace(@Nested) private public class Outer {}", "public", "@Trace(@Nested)"),
        (
            "kotlin",
            "@Trace(\"café\")\ninternal class Outer {\n @Inner\n private fun hidden() {}\n}\n",
            "internal",
            "@Trace(\"café\")",
        ),
        ("kotlin", "@Trace\nprivate internal class Outer {}\n", "internal", "@Trace"),
    ] {
        let tree = parse(language, source.as_bytes())?;
        let node = crate::syntax::walk(tree.root_node()).find(|node| node.kind() == "class_declaration").unwrap();
        let expected = (Some(visibility.to_owned()), vec![annotation.to_owned()]);
        assert_eq!(
            modifier_metadata((node, source.as_bytes()), &["public", "private", "protected", "internal"], None),
            expected,
        );
        assert_eq!(
            modifier_metadata((node, source.as_bytes()), &[], queries::metadata(language)),
            ((language == "kotlin").then(|| visibility.to_owned()), vec![annotation.to_owned()]),
        );
        assert_eq!(
            get_adapter(language)?.metadata(node, source.as_bytes()),
            expected,
            "{language}: {}",
            tree.root_node().to_sexp()
        );
    }
    Ok(())
}

#[test]
fn inheritance_queries_preserve_header_policies_and_reparsing() -> Result<()> {
    use crate::parsing::{
        adapters::get_adapter,
        syntax::{DeclarationHeader, inheritance_names},
    };
    for (language, kind, signature, expected) in [
        ("python", "class", "class Child(pkg.Base[T], Other, metaclass=Meta):", vec!["pkg.Base", "Other"]),
        ("javascript", "class", "class Child extends ns.Base", vec!["ns.Base"]),
        ("jsx", "class", "class Child extends ns.Base {}", vec![]),
        (
            "typescript",
            "class",
            "class Child<T> extends ns.Base<T> implements Reader<T>, Writer",
            vec!["ns.Base", "Reader", "Writer"],
        ),
        ("tsx", "interface", "interface Child<T> extends ns.Reader<T>, Writer", vec!["ns.Reader", "Writer"]),
        (
            "java",
            "class",
            "class Child<T> extends pkg.Base<T> implements Reader<T>, Writer",
            vec!["pkg.Base", "Reader", "Writer"],
        ),
        ("java", "interface", "interface Child<T> extends Reader<T>, pkg.Writer", vec!["Reader", "pkg.Writer"]),
        ("kotlin", "class", "class Child<T> : pkg.Base<T>(), Reader<T>", vec!["pkg.Base", "Reader"]),
        (
            "typescript",
            "class",
            "class Outer extends Root { method() { class Inner extends Hidden {} } }",
            vec!["Root"],
        ),
        ("python", "class", "class Outer(Root):\n class Inner(Hidden): pass", vec!["Root"]),
    ] {
        let adapter = get_adapter(language)?;
        let header = || DeclarationHeader { language, kind, signature };
        assert_eq!(adapter.inheritance(header()), expected, "{language}: {signature}");
        let mut legacy = adapter.inheritance_policy();
        legacy.query = None;
        assert_eq!(inheritance_names(header(), legacy), expected, "legacy {language}: {signature}");
    }
    Ok(())
}

#[test]
fn supplemental_imports_keep_native_text_and_deduplicate_by_line() -> Result<()> {
    use crate::parsing::{adapters::get_adapter, syntax::parse};
    let source = b"\nmod child;\nmod child;\nmod inline {}\n";
    let tree = parse("rust", source)?;
    let native = vec![("native".to_owned(), 9), (" mod child; ".to_owned(), 2)];
    assert_eq!(
        get_adapter("rust")?.imports(tree.root_node(), source, native),
        [(" mod child; ".to_owned(), 2), ("mod child;".to_owned(), 3), ("native".to_owned(), 9)]
    );
    Ok(())
}

#[test]
fn unsupported_grammars_are_rejected() {
    for language in ["lua", "json", "csv", "c", "cpp", "csharp", "php", "ruby"] {
        assert!(parse(language, "").is_err(), "{language}");
    }
}
