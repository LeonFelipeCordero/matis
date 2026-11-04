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
fn unsupported_grammars_are_rejected() {
    for language in ["lua", "json", "csv", "c", "cpp", "csharp", "php", "ruby"] {
        assert!(parse(language, "").is_err(), "{language}");
    }
}
