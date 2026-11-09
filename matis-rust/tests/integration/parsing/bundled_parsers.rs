//! Installed Matis must analyze every supported language without parser setup.
use matis::parsing::discovery::SUPPORTED_LANGUAGES;
use serde_json::Value;
use std::{collections::BTreeSet, fs, process::Command};

#[test]
fn build_rejects_missing_or_dynamic_parser_configuration() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let guard = workspace.path().join(format!("parser-build-guard{}", std::env::consts::EXE_SUFFIX));
    let compiled = Command::new("rustc")
        .args(["--edition=2024", "--crate-name", "parser_build_guard"])
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
        .arg("-o")
        .arg(&guard)
        .output()?;
    assert!(compiled.status.success(), "{compiled:?}");
    let languages =
        SUPPORTED_LANGUAGES.iter().copied().filter(|language| *language != "jsx").collect::<Vec<_>>().join(",");
    for (selected, mode, succeeds) in [
        ("", "", false),
        ("python", "static", false),
        (languages.as_str(), "dynamic", false),
        (languages.as_str(), "static", true),
        (" ALL ", "static", true),
        ("*", "static", true),
    ] {
        let output = Command::new(&guard)
            .current_dir(workspace.path())
            .env_clear()
            .env("TSLP_LANGUAGES", selected)
            .env("TSLP_LINK_MODE", mode)
            .output()?;
        assert_eq!(output.status.success(), succeeds, "{output:?}");
        if !succeeds {
            assert!(String::from_utf8_lossy(&output.stderr).contains("Build from the repository root"), "{output:?}");
        }
    }
    Ok(())
}

#[test]
fn relocated_binary_analyzes_all_languages_without_external_parsers() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let root = workspace.path().join("repo");
    fs::create_dir(&root)?;
    let samples = [
        ("main.py", "def run():\n    return 1\n"),
        ("main.js", "function run() { return 1; }\n"),
        ("main.jsx", "function Run() { return <div />; }\n"),
        ("main.ts", "function run(): number { return 1; }\n"),
        ("main.tsx", "function Run() { return <div />; }\n"),
        ("Main.java", "class Main { int run() { return 1; } }\n"),
        ("main.kt", "fun run(): Int { return 1 }\n"),
        ("main.go", "package main\nfunc run() int { return 1 }\n"),
        ("main.rs", "fn run() -> i32 { 1 }\n"),
        ("main.ex", "defmodule Main do\n  def run(), do: 1\nend\n"),
    ];
    assert_eq!(samples.len(), SUPPORTED_LANGUAGES.len());
    for (name, source) in samples {
        fs::write(root.join(name), source)?;
    }
    fs::write(root.join("matis.json"), r#"{"maxFileLines":0}"#)?;
    let binary = workspace.path().join(format!("matis{}", std::env::consts::EXE_SUFFIX));
    fs::copy(env!("CARGO_BIN_EXE_matis"), &binary)?;
    let cache = workspace.path().join("cache");
    let output = Command::new(binary)
        .current_dir(workspace.path())
        .env_clear()
        .env("HOME", workspace.path())
        .env("XDG_CACHE_HOME", &cache)
        .env("PATH", "")
        .env("TREE_SITTER_LANGUAGE_PACK_MANIFEST_URL", "http://127.0.0.1:1/parsers.json")
        .env("TREE_SITTER_LANGUAGE_PACK_LIBS_DIR", workspace.path().join("absent-parsers"))
        .args(["check", "--folder", root.to_str().unwrap(), "--format", "json"])
        .output()?;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let findings = report["findings"].as_array().unwrap();
    assert!(findings.iter().all(|finding| finding["rule"] == "maxFileLines"), "{report}");
    let files: BTreeSet<_> = findings.iter().map(|finding| finding["subject"].as_str().unwrap()).collect();
    assert_eq!(files, samples.iter().map(|(name, _)| *name).collect());
    assert!(!cache.exists(), "analysis must not create a parser cache");
    Ok(())
}
