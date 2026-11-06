use matis::metrics::{
    check::build_check,
    config::{validate_config, validate_project_config},
};
use serde_json::{Value, json};

fn node(path: &str, level: &str, (incoming, outgoing): (u32, u32)) -> Value {
    json!({"subject": path, "path": path, "kind": level, "level": level, "line": 1,
        "metrics": {"fanIn": incoming, "fanOut": outgoing, "lineCount": 701,
            "transitiveChangeImpact": 2, "instability": 0.5}})
}

fn report(nodes: Vec<Value>) -> Value {
    json!({"nodes": nodes, "summary": {"circularImportGroups": 0}, "circularImports": [], "diagnostics": []})
}

fn rules(report: &Value, config: Value) -> Vec<String> {
    build_check(report, &config).unwrap()["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["rule"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn file_fan_limits_are_inclusive_and_nullable() {
    for (incoming, outgoing, expected) in [
        (30, 20, vec![]),
        (31, 20, vec!["maxFanIn"]),
        (30, 21, vec!["maxFanOut"]),
        (31, 21, vec!["maxFanIn", "maxFanOut"]),
    ] {
        let report = report(vec![node("a.py", "file", (incoming, outgoing))]);
        assert_eq!(rules(&report, json!({"maxFileLines": null})), expected);
        assert!(rules(&report, json!({"maxFileLines": null, "maxFanIn": null, "maxFanOut": null})).is_empty());
    }
    let report = report(vec![node("a.py", "file", (1, 1))]);
    assert_eq!(rules(&report, json!({"maxFileLines": null, "maxFanIn": 0, "maxFanOut": 0})), ["maxFanIn", "maxFanOut"]);
}

#[test]
fn fan_rules_do_not_apply_to_modules_or_change_existing_module_rules() {
    let report = report(vec![node("src", "module", (31, 21))]);
    assert!(rules(&report, json!({})).is_empty());
    assert_eq!(
        rules(&report, json!({"fanExclude": ["**"], "maxChangeImpact": 1, "maxInstability": 0})),
        ["maxChangeImpact", "maxInstability"]
    );
}

#[test]
fn exclusions_use_root_relative_gitignore_rules_without_skipping_other_checks() {
    for (patterns, path, excluded) in [
        (json!(["/src/a.py"]), "src/a.py", true),
        (json!(["/src/a.py"]), "nested/src/a.py", false),
        (json!(["src/**/*.py"]), "src/nested/a.py", true),
        (json!(["src/"]), "src/nested/a.py", true),
        (json!(["*.py", "!src/a.py"]), "src/a.py", false),
        (json!(["*.py", "!src/a.py", "src/a.py"]), "src/a.py", true),
    ] {
        let mut report = report(vec![node(path, "file", (31, 21))]);
        report["diagnostics"] = json!([{"path": path, "message": "Parse failed"}]);
        let before = report.clone();
        let actual = rules(&report, json!({"fanExclude": patterns, "maxChangeImpact": 1, "maxInstability": 0}));
        let mut expected = vec!["maxFileLines", "maxChangeImpact", "maxInstability"];
        if !excluded {
            expected.extend(["maxFanIn", "maxFanOut"]);
        }
        expected.push("analysis");
        assert_eq!(actual, expected, "{path}: {patterns}");
        assert_eq!(report, before);
    }
}

#[test]
fn excluded_files_still_contribute_to_peer_counts() {
    use matis::metrics::dependency_metrics::dependency_metrics;
    let metrics = dependency_metrics(&json!({"nodes": ["src/a.py", "src/b.py"], "edges": [
        {"sourceStableKey": "src/a.py", "targetStableKey": "src/b.py"}
    ]}));
    let nodes = metrics
        .into_iter()
        .map(|(path, metrics)| {
            let mut node = node(&path, "file", (0, 0));
            node["metrics"] = metrics;
            node
        })
        .collect();
    let check = build_check(
        &report(nodes),
        &json!({"maxFileLines": null,
        "maxFanIn": 0, "maxFanOut": 0, "fanExclude": ["src/a.py"]}),
    )
    .unwrap();
    assert_eq!(check["findings"].as_array().unwrap().len(), 1);
    assert_eq!(check["findings"][0]["rule"], "maxFanIn");
    assert_eq!(check["findings"][0]["subject"], "src/b.py");
    assert_eq!(check["findings"][0]["actual"], 1);
}

#[test]
fn fan_config_defaults_and_validation_apply_at_both_boundaries() {
    for validate in [validate_config, validate_project_config] {
        let defaults = validate(&json!({})).unwrap();
        assert_eq!(defaults["maxFanIn"], 30);
        assert_eq!(defaults["maxFanOut"], 20);
        assert_eq!(defaults["fanExclude"], json!([]));
    }
    for rule in ["maxFanIn", "maxFanOut"] {
        check_limit_config(rule);
    }
    for invalid in [
        json!(null),
        json!("*.py"),
        json!({}),
        json!([1]),
        json!([true]),
        json!(["!"]),
        json!(["bad\\"]),
        json!(["[z-a]"]),
    ] {
        assert_invalid_config(&json!({"fanExclude": invalid}), "fanExclude");
    }
}

fn check_limit_config(rule: &str) {
    for valid in [json!(null), json!(0), json!(30)] {
        assert_eq!(validate_config(&json!({rule: valid})).unwrap()[rule], valid);
        assert_eq!(validate_project_config(&json!({rule: valid})).unwrap()[rule], valid);
    }
    for invalid in [json!(-1), json!(1.0), json!(true), json!("30"), json!([])] {
        assert_invalid_config(&json!({rule: invalid}), rule);
    }
}

fn assert_invalid_config(config: &Value, field: &str) {
    assert!(validate_config(config).unwrap_err().to_string().contains(field));
    assert!(validate_project_config(config).unwrap_err().to_string().contains(field));
}
