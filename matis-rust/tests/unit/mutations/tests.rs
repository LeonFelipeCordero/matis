use super::*;
use crate::{domain::models::SourceFile, parsing::mutation_candidates::extract_candidates};
use anyhow::Result;
use serde_json::json;
use std::collections::BTreeMap;

fn source(path: &str, language: &str, data: &[u8]) -> SourceFile {
    SourceFile { path: path.into(), absolute_path: path.into(), language: language.into(), source: data.to_vec() }
}

#[test]
fn canonical_json_sorts_nested_keys_and_preserves_unicode() {
    assert_eq!(
        canonical_json(&json!({"z": [2, {"é": "é", "a": 3}], "a": true})),
        "{\"a\":true,\"z\":[2,{\"a\":3,\"é\":\"é\"}]}"
    );
}

#[test]
fn strict_config_and_catalog() -> Result<()> {
    assert_eq!(rules::REGISTRY.len(), 24);
    let counts: BTreeMap<_, _> = rules::LANGUAGES
        .iter()
        .map(|language| {
            (
                *language,
                rules::REGISTRY
                    .iter()
                    .flat_map(|rule| &rule.mappings)
                    .filter(|mapping| mapping.language == *language)
                    .count(),
            )
        })
        .collect();
    assert_eq!(counts["elixir"], 18);
    assert_eq!(counts["python"], 18);
    assert_eq!(counts["javascript"], 20);
    assert_eq!(config::MutationConfig::from_value(&json!({}))?.rules, Vec::<String>::new());
    let selected = json!({"rules": ["logical.or-to-and", "logical.or-to-and"]});
    assert_eq!(config::MutationConfig::from_value(&selected)?.rules.len(), 2);
    for invalid in [
        json!(null),
        json!([]),
        json!({"rules": null}),
        json!({"rules": {}}),
        json!({"rules": [1]}),
        json!({"rules": ["unknown"]}),
        json!({"replacement": "+"}),
    ] {
        assert!(config::MutationConfig::from_value(&invalid).is_err(), "{invalid}");
    }
    assert!(rules::validate_rules_value(&json!({})).unwrap_err().to_string().contains("must be a list"));
    assert!(rules::validate_rules(&["unknown".into()]).is_err());
    Ok(())
}

#[test]
fn reject_invalid_bytes_errors_and_excluded_syntax() -> Result<()> {
    for (language, data) in [
        ("unknown", b"a+b".as_slice()),
        ("python", b"\xff+a"),
        ("python", b"if x > :\n"),
        ("python", b"# a+b\nx = 'a+b'\n"),
        ("rust", b"fn test(p: &i32) { let a = *p; }"),
        ("elixir", b"quote do: a + b\ndefmacro foo(a), do: a + b\n"),
    ] {
        assert!(extract_candidates(&source("sample", language, data))?.is_empty(), "{language}: {data:?}");
    }
    Ok(())
}
