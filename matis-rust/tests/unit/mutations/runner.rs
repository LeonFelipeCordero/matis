use super::{config::MutationConfig, matching};
use anyhow::Result;
use serde_json::{Value, json};

#[test]
fn rooted_globs_and_character_classes() {
    for (pattern, path, expected) in [
        ("src/module/**", "src/module/a/file.py", true),
        ("src/module/**", "src/module_other/file.py", false),
        ("src/*.py", "src/a/file.py", false),
        ("src/*.py", "src/file.py", true),
        ("src/**/file.py", "src/file.py", true),
        ("src/**/file.py", "src/a/b/file.py", true),
        ("src/**/right.py", "src/a/wrong.py", false),
        ("src/test_?.py", "src/test_a.py", true),
        ("src/[ab].py", "src/c.py", false),
        ("[!a-c]", "d", true),
        ("[!a-c]", "b", false),
        ("[]]", "]", true),
        ("[", "[", true),
        ("*", ".hidden", true),
    ] {
        assert_eq!(matching::matches(path, pattern), expected, "{path}: {pattern}");
    }
}

#[test]
fn targets_validate_strictly_and_retain_order() {
    let settings = json!({"sources": ["src/**"], "command": ["true"]});
    for (key, bad) in [
        ("sources", json!([])),
        ("sources", json!([""])),
        ("sources", json!(["../escape/**"])),
        ("sources", json!(["/absolute/**"])),
        ("sources", json!(["a\\b"])),
        ("command", json!([])),
        ("command", json!([" "])),
        ("timeoutSeconds", json!(0)),
        ("timeoutSeconds", json!(true)),
        ("timeoutSeconds", json!(1.0)),
        ("cwd", Value::Null),
        ("extra", json!(1)),
    ] {
        let mut input = settings.clone();
        input[key] = bad;
        assert!(MutationConfig::from_value(&json!({"targets": {"unit": input}})).is_err(), "{key}");
    }
    for name in ["", "all", " unit", "unit ", "a,b"] {
        assert!(MutationConfig::from_value(&json!({"targets": {name: settings}})).is_err());
    }
    let settings = MutationConfig::from_value(&json!({"targets": {"z": settings.clone(), "a": settings}})).unwrap();
    assert_eq!(settings.targets.keys().collect::<Vec<_>>(), vec!["z", "a"]);
    assert_eq!(settings.targets["z"].timeout_seconds.as_u64(), Some(120));
    let huge = serde_json::from_str(
        r#"{"targets":{"unit":{"sources":["*"],"command":["true"],"timeoutSeconds":999999999999999999999999999999}}}"#,
    )
    .unwrap();
    assert!(MutationConfig::from_value(&huge).is_ok());
}

#[test]
fn combined_outcomes_obey_priority_in_either_order() {
    // Real timeout and process cleanup are covered in runner_process_integration_tests.
    for (first, second, expected) in [
        ("survived", "survived", "survived"),
        ("survived", "error", "error"),
        ("survived", "timeout", "timeout"),
        ("timeout", "error", "error"),
        ("killed", "error", "killed"),
        ("timeout", "killed", "killed"),
    ] {
        let mut targets = [first, second].map(|outcome| json!({"outcome": outcome}));
        assert_eq!(super::runner::combined_outcome(&targets), expected);
        targets.reverse();
        assert_eq!(super::runner::combined_outcome(&targets), expected);
    }
}

#[test]
fn project_config_normalizes_targets_and_reports_nested_fields() -> Result<()> {
    use crate::metrics::config::validate_project_config;
    let value = validate_project_config(&json!({"mutations": {"targets": {
        "unit": {"sources": ["src/**"], "command": ["true"]}
    }}}))?;
    assert_eq!(value["mutations"]["targets"]["unit"]["cwd"], ".");
    assert_eq!(value["mutations"]["targets"]["unit"]["timeoutSeconds"], 120);
    for (key, bad, expected) in [
        ("sources", json!([]), "too_short"),
        ("command", json!([false]), "string_type"),
        ("timeoutSeconds", json!(true), "int_type"),
        ("timeoutSeconds", json!(0), "greater_than"),
        ("extra", json!(1), "extra_forbidden"),
    ] {
        let mut target = json!({"sources": ["src/**"], "command": ["true"]});
        target[key] = bad;
        let error =
            validate_project_config(&json!({"mutations": {"targets": {"unit": target}}})).unwrap_err().to_string();
        assert!(error.contains(&format!("mutations.targets.unit.{key}")));
        assert!(error.contains(&format!("type={expected}")));
    }
    Ok(())
}
