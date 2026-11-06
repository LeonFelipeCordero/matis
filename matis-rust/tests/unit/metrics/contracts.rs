use matis::{
    domain::models::{ParsedFile, ParsedRepository, SourceFile, Symbol},
    metrics::{
        adapters::base::MetricPolicy,
        artifact::{build_artifact, build_dependency_views, validate_artifact, validate_references},
        callables::callable_metrics,
        check::{build_check, render_check},
        complexity::calculate_complexity,
        config::{default_config, load_config, load_project_config, validate_config, validate_project_config},
        cycles::circular_dependencies,
        dependency_metrics::dependency_metrics,
        report::{build_artifact_report, render_markdown},
        resolver::{Connection, ResolvedGraph},
    },
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

#[path = "isolated.rs"]
mod isolated;

fn node(key: &str, level: &str, parent: Option<&str>) -> Value {
    json!({"stableKey": key, "type": level, "kind": level, "level": level, "parentStableKey": parent,
        "name": key, "qualifiedName": key, "path": key, "properties": {}})
}
fn edge(source: &str, target: &str, kind: &str) -> Value {
    json!({"sourceStableKey": source, "targetStableKey": target, "type": kind, "confidence": "EXTRACTED"})
}
fn file(path: &str, language: &str, source: &[u8]) -> ParsedFile {
    ParsedFile {
        source: SourceFile {
            path: path.into(),
            absolute_path: Path::new("/repo").join(path),
            language: language.into(),
            source: source.into(),
        },
        content_hash: "digest".into(),
        parse_error: None,
        symbols: vec![],
        imports: vec![],
        calls: vec![],
        inheritances: vec![],
    }
}
fn parsed_symbol(root: tree_sitter::Node<'_>, kind: &str) -> Symbol {
    Symbol {
        key: "fn".into(),
        file_path: "a.py".into(),
        language: "python".into(),
        name: "brew".into(),
        qualified_name: "a.brew".into(),
        kind: kind.into(),
        parent_key: "file:a.py".into(),
        start_byte: root.start_byte(),
        end_byte: root.end_byte(),
        start_line: root.start_position().row + 1,
        start_column: root.start_position().column + 1,
        end_line: root.end_position().row + 1,
        end_column: root.end_position().column + 1,
        signature: None,
        visibility: None,
        decorators: vec![],
        complexity: None,
        line_count: None,
        parameter_count: None,
    }
}
fn parse(language: &str, source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_language_pack::get_language(language).unwrap()).unwrap();
    parser.parse(source, None).unwrap()
}

#[test]
fn empty_artifact_has_root_hierarchy_and_exact_counts() {
    let artifact = build_artifact(
        Path::new("/repo"),
        &ParsedRepository::default(),
        ResolvedGraph { connections: vec![], external_nodes: vec![] },
    )
    .unwrap();
    assert_eq!(
        artifact["stats"],
        json!({"files": 0, "symbols": 0, "modules": 1, "types": 0,
        "callables": 0, "nodes": 2, "edges": 1, "externalNodes": 0, "parseErrors": 0})
    );
    assert_eq!(artifact["nodes"][1]["ancestorStableKeys"], json!(["repository"]));
    assert_eq!(
        artifact["nodes"][1]["properties"],
        json!({"fanIn": 0, "fanOut": 0, "transitiveChangeImpact": 0, "instability": null})
    );
    assert_eq!(artifact["circularDependencies"], json!({"version": 1, "groups": []}));
    assert!(artifact["repository"]["indexedAt"].as_str().unwrap().ends_with("+00:00"));
    validate_artifact(&artifact).unwrap();
}

#[test]
fn file_lines_and_deterministic_module_order() {
    let files =
        ["z/q/a.py", "a/b.py", "z/q/c.py", "root.py"].into_iter().map(|path| file(path, "python", b"a\nb")).collect();
    let artifact = build_artifact(
        Path::new("/repo"),
        &ParsedRepository { files },
        ResolvedGraph { connections: vec![], external_nodes: vec![] },
    )
    .unwrap();
    let modules: Vec<_> = artifact["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["level"] == "module")
        .map(|node| node["stableKey"].clone())
        .collect();
    assert_eq!(modules, json!(["module:.", "module:a", "module:z", "module:z/q"]).as_array().unwrap().clone());
    let first_file = &artifact["nodes"][5];
    assert_eq!(first_file["properties"]["lineCount"], 2);
    assert_eq!(first_file["properties"]["bytes"], 3);
    assert_eq!(first_file["ancestorStableKeys"], json!(["repository", "module:.", "module:z", "module:z/q"]));
}

#[test]
fn artifact_enriches_connections_and_rechecks_in_place_edits() {
    let repo = ParsedRepository { files: vec![file("a.py", "python", b""), file("b.py", "python", b"")] };
    let connection = Connection {
        source_key: "file:a.py".into(),
        target_key: "file:b.py".into(),
        type_: "IMPORTS".into(),
        confidence: "EXTRACTED".into(),
        path: "a.py".into(),
        line: 1,
        evidence: None,
        type_only: true,
    };
    let mut artifact = build_artifact(
        Path::new("/repo"),
        &repo,
        ResolvedGraph { connections: vec![connection], external_nodes: vec![] },
    )
    .unwrap();
    assert_eq!(artifact["edges"].as_array().unwrap().last().unwrap()["sourceLevel"], "file");
    assert_eq!(artifact["nodes"][2]["properties"]["instability"], json!(1.0));
    artifact["nodes"][0]["properties"]["fanIn"] = json!(true);
    assert!(validate_references(&artifact).unwrap_err().to_string().contains("nonnegative integer"));
    artifact["nodes"][0]["properties"].as_object_mut().unwrap().remove("fanIn");
    artifact["nodes"][0]["unexpected"] = json!(1);
    assert!(validate_artifact(&artifact).is_err());
}

#[test]
fn views_project_nearest_ancestors_and_preserve_node_order() {
    let nodes = vec![
        node("m", "module", None),
        node("b", "file", Some("m")),
        node("a", "file", Some("m")),
        node("class", "type", Some("a")),
        node("fn", "callable", Some("class")),
        node("fn2", "callable", Some("class")),
    ];
    let mut inferred = edge("fn", "b", "CALLS");
    inferred["confidence"] = json!("INFERRED");
    let edges = vec![
        inferred,
        edge("a", "b", "IMPORTS"),
        edge("fn", "fn2", "CALLS"),
        edge("b", "a", "REFERENCES"),
        edge("a", "b", "CONTAINS"),
    ];
    let views = build_dependency_views(&nodes, &edges);
    assert_eq!(views["file"]["nodes"], json!(["b", "a"]));
    assert_eq!(
        views["file"]["edges"][0],
        json!({"sourceStableKey": "a", "targetStableKey": "b", "type": "DEPENDS_ON", "level": "file", "count": 2,
        "relationshipCounts": {"CALLS": 1, "IMPORTS": 1}, "confidenceCounts": {"EXTRACTED": 1, "INFERRED": 1}})
    );
    assert_eq!(views["module"]["edges"], json!([]));
    assert_eq!(views["callable"]["edges"][0]["targetStableKey"], "fn2");
}

#[test]
fn dependency_metrics_deduplicate_diamonds_cycles_and_self_edges() {
    let edges = vec![
        edge("a", "b", "DEPENDS_ON"),
        edge("a", "c", "DEPENDS_ON"),
        edge("b", "d", "DEPENDS_ON"),
        edge("c", "d", "DEPENDS_ON"),
        edge("d", "c", "DEPENDS_ON"),
        edge("a", "a", "DEPENDS_ON"),
        edge("a", "b", "DEPENDS_ON"),
    ];
    let result = dependency_metrics(&json!({"nodes": ["a", "b", "c", "d", "isolated"], "edges": edges}));
    assert_eq!(result["d"]["transitiveChangeImpact"], 3);
    assert_eq!(result["c"]["transitiveChangeImpact"], 3);
    assert_eq!(result["a"]["fanOut"], 2);
    assert_eq!(result["isolated"], json!({"fanIn": 0, "fanOut": 0, "transitiveChangeImpact": 0, "instability": null}));
}

#[test]
fn cycles_project_symbol_imports_and_exclude_type_only_and_symbol_self_imports() {
    let nodes =
        vec![node("file:a", "file", None), node("file:b", "file", None), node("fn", "callable", Some("file:a"))];
    let edges =
        vec![edge("fn", "file:b", "IMPORTS"), edge("file:b", "file:a", "IMPORTS"), edge("fn", "file:a", "IMPORTS")];
    let result = circular_dependencies(&nodes, &edges);
    assert_eq!(result["groups"][0]["nodeStableKeys"], json!(["file:a", "file:b"]));
    assert_eq!(result["groups"][0]["exampleCycle"], json!(["file:a", "file:b", "file:a"]));
    assert_eq!(result["groups"][0]["edges"].as_array().unwrap().len(), 2);
    let mut excluded = edges.clone();
    excluded[1]["typeOnly"] = json!(true);
    assert_eq!(circular_dependencies(&nodes, &excluded), json!({"version": 1, "groups": []}));
}

#[test]
fn cycle_identifiers_match_python_ascii_json_hash() {
    let nodes = vec![node("file:é🦀", "file", None)];
    let result = circular_dependencies(&nodes, &[edge("file:é🦀", "file:é🦀", "IMPORTS")]);
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(br#"["file:\u00e9\ud83e\udd80"]"#));
    assert_eq!(result["groups"][0]["id"], format!("import-cycle:{digest}"));
}

#[test]
fn callable_metrics_skip_nested_scopes_and_count_parameters_and_lines() {
    let source = b"def brew(a, b):\n    for x in a:\n        while b:\n            if x:\n                pass\n    def nested():\n        if b:\n            pass\n";
    let tree = parse("python", source);
    let root = tree.root_node().named_child(0).unwrap();
    let mut file = file("a.py", "python", source);
    file.symbols.push(parsed_symbol(root, "function"));
    callable_metrics(&mut file).unwrap();
    let symbol = &file.symbols[0];
    assert_eq!(symbol.parameter_count, Some(2));
    assert_eq!(symbol.line_count, Some(8));
    let complexity = symbol.complexity.as_ref().unwrap();
    assert_eq!(complexity.base_cyclomatic_complexity, 4);
    assert_eq!(complexity.nested_cyclomatic_complexity, 1);
    assert_eq!(complexity.loop_nested_complexity, 1);
    assert_eq!(complexity.cyclomatic_complexity, 6);
    assert_eq!(complexity.max_decision_depth, 3);
}

#[test]
fn alternatives_share_decision_depth() {
    let source = b"def brew(a):\n    if a:\n        pass\n    elif a:\n        pass\n    elif a:\n        pass\n";
    let tree = parse("python", source);
    let complexity =
        calculate_complexity(tree.root_node().named_child(0).unwrap(), &MetricPolicy::new("python"), source);
    assert_eq!(complexity.base_cyclomatic_complexity, 4);
    assert_eq!(complexity.max_decision_depth, 1);
}

#[test]
fn configs_keep_mutation_settings_separate_and_reject_coercion() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(load_config(root.path()).unwrap(), default_config());
    std::fs::write(
        root.path().join("matis.json"),
        r#"{"maxFileLines":null,"maxInstability":0,"mutations":{"rules":["logical.and-to-or"]}}"#,
    )
    .unwrap();
    let config = load_config(root.path()).unwrap();
    assert!(config.get("mutations").is_none());
    assert_eq!(config["maxInstability"], json!(0.0));
    assert_eq!(load_project_config(root.path()).unwrap()["mutations"]["rules"], json!(["logical.and-to-or"]));
}

#[test]
fn reports_recompute_dependencies_preserve_zeros_and_render_exact_markdown() {
    let mut file = node("file:a", "file", None);
    file["path"] = json!("a.py");
    file["qualifiedName"] = json!("a.py");
    file["properties"]["lineCount"] = json!(0);
    let report = build_artifact_report(&json!({"nodes": [file], "edges": []})).unwrap();
    assert_eq!(
        report["nodes"][0]["metrics"],
        json!({"lineCount": 0, "functionCount": 0, "measuredFunctionCount": 0, "weightedComplexitySum": 0,
        "fanIn": 0, "fanOut": 0, "transitiveChangeImpact": 0, "instability": null})
    );
    assert_eq!(
        render_markdown(&report),
        concat!(
            "# Metrics\n\n- files: 1\n- modules: 0\n- types: 0\n- callables: 0\n- circularImportGroups: 0\n- parseErrors: 0\n\n",
            "## Files\n\n| Item | Location | Lines | Functions/methods | Measured functions/methods | Weighted complexity sum |\n| --- | --- | --- | --- | --- | --- |\n| `a.py` | `a.py` | 0 | 0 | 0 | 0 |\n\n",
            "## Dependencies\n\n| Item | Location | Fan-in | Fan-out | Change impact | Internal instability |\n| --- | --- | --- | --- | --- | --- |\n| `a.py` | `a.py` | 0 | 0 | 0 | N/A |\n\n",
            "Dependency metrics count distinct internal peers at each item's level, including tests and type-only dependencies. Impact excludes the item itself. Instability = fan-out / (fan-in + fan-out); isolated items are N/A."
        )
    );
}

#[test]
fn checks_report_missing_enabled_metrics_but_skip_isolated_instability() {
    let mut config: Value =
        Value::Object(default_config().as_object().unwrap().keys().map(|key| (key.clone(), Value::Null)).collect());
    config["fanExclude"] = json!([]);
    config["maxParameters"] = json!(0);
    config["maxInstability"] = json!(0.0);
    let report = json!({"nodes": [{"subject": "a.brew", "kind": "function", "level": "callable", "path": "a.py", "line": 2, "metrics": {}},
        {"subject": "a.py", "kind": "file", "level": "file", "metrics": {"fanIn": 0, "fanOut": 0}}],
        "summary": {"circularImportGroups": 0}, "circularImports": [], "diagnostics": []});
    let check = build_check(&report, &config).unwrap();
    assert_eq!(check["findings"].as_array().unwrap().len(), 1);
    assert_eq!(
        render_check(&check),
        "## maxParameters: `a.brew`\n\nMeasured: N/A; maximum: 0.\n\n- `a.py:2`\n\n- Parameters is unavailable; cannot evaluate maximum 0."
    );
    assert_eq!(render_check(&json!({"findings": []})), "All checks passed.");
}

fn count_check(actual: Value, maximum: Value) -> Value {
    let mut config = default_config();
    for value in config.as_object_mut().unwrap().values_mut() {
        *value = Value::Null;
    }
    config["fanExclude"] = json!([]);
    config["maxFileLines"] = maximum;
    let report = json!({"summary": {"circularImportGroups": 0}, "circularImports": [], "diagnostics": [],
        "nodes": [{"subject": "a.py", "kind": "file", "level": "file", "path": "a.py", "line": null,
            "metrics": {"lineCount": actual}}]});
    build_check(&report, &config).unwrap()
}
fn invalid_project(root: &Path, source: &str) -> String {
    std::fs::write(root.join("matis.json"), source).unwrap();
    load_project_config(root).unwrap_err().to_string()
}

#[test]
fn arbitrary_size_limits_remain_exact_json_integers() {
    let integer = "9".repeat(400);
    let number: Value = serde_json::from_str(&integer).unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("matis.json"), format!("{{\"maxFileLines\":{integer}}}")).unwrap();
    assert_eq!(load_config(root.path()).unwrap()["maxFileLines"].to_string(), integer);
    for rule in [
        "maxFileLines",
        "maxFunctionLines",
        "maxParameters",
        "maxWeightedComplexity",
        "maxDecisionDepth",
        "maxNestedLoops",
        "maxCircularImportGroups",
        "maxChangeImpact",
        "maxFanIn",
        "maxFanOut",
    ] {
        assert_eq!(validate_config(&json!({rule: number})).unwrap()[rule], number);
    }
    assert_eq!(count_check(number.clone(), number)["findings"], json!([]));
}

#[test]
fn integer_comparisons_do_not_round_above_u64_or_f64_precision() {
    let maximum: Value = serde_json::from_str("18446744073709551616").unwrap();
    let above: Value = serde_json::from_str("18446744073709551617").unwrap();
    let finding = count_check(above.clone(), maximum.clone());
    assert_eq!(finding["findings"][0]["actual"], above);
    assert_eq!(finding["findings"][0]["limit"], maximum);
    assert_eq!(
        count_check(json!(18446744073709551616.0), json!(18446744073709551615u64))["findings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(count_check(json!(1.5), json!(1))["findings"].as_array().unwrap().len(), 1);
    assert_eq!(count_check(json!(-1.5), json!(0))["findings"], json!([]));
}

#[test]
fn arbitrary_cycle_group_limits_do_not_require_u64() {
    let maximum: Value = serde_json::from_str(&"9".repeat(400)).unwrap();
    let report = json!({"nodes": [], "summary": {"circularImportGroups": 1}, "circularImports": [], "diagnostics": []});
    assert_eq!(build_check(&report, &json!({"maxCircularImportGroups": maximum})).unwrap(), json!({"findings": []}));
}

#[test]
fn negative_large_integers_and_fractional_limits_have_exact_python_errors() {
    let root = tempfile::tempdir().unwrap();
    let negative = format!("-{}", "9".repeat(100));
    let error = invalid_project(root.path(), &format!("{{\"maxFileLines\":{negative}}}"));
    let abbreviated = format!("-{}...{}", "9".repeat(24), "9".repeat(24));
    assert!(error.contains(&format!("[type=greater_than_equal, input_value={abbreviated}, input_type=int]")));
    let fractional = invalid_project(root.path(), "{\"maxFileLines\":18446744073709551616.5}");
    assert!(fractional.contains("[type=int_type, input_value=1.8446744073709552e+19, input_type=float]"));
    let overflow = invalid_project(root.path(), "{\"maxFileLines\":1e400}");
    assert!(overflow.contains("[type=int_type, input_value=inf, input_type=float]"));
}

#[test]
fn nonfinite_json_tokens_match_python_validation_diagnostics() {
    let root = tempfile::tempdir().unwrap();
    for (token, representation) in [("NaN", "nan"), ("Infinity", "inf"), ("-Infinity", "-inf")] {
        let error = invalid_project(root.path(), &format!("{{\"maxInstability\":{token}}}"));
        assert_eq!(
            error,
            format!(
                "Invalid {}: 1 validation error for ProjectConfig\nmaxInstability\n  Input should be a finite number [type=finite_number, input_value={representation}, input_type=float]\n    For further information visit https://errors.pydantic.dev/2.14/v/finite_number",
                root.path().join("matis.json").display()
            )
        );
    }
    assert!(
        invalid_project(root.path(), "{\"maxParameters\":NaN}")
            .contains("[type=int_type, input_value=nan, input_type=float]")
    );
    assert!(
        invalid_project(root.path(), "{\"mutations\":{\"rules\":[Infinity]}}")
            .contains("[type=string_type, input_value=inf, input_type=float]")
    );
}

#[test]
fn quoted_tokens_and_internal_marker_strings_remain_strings() {
    let root = tempfile::tempdir().unwrap();
    assert!(
        invalid_project(root.path(), r#"{"maxInstability":"NaN"}"#)
            .contains("[type=float_type, input_value='NaN', input_type=str]")
    );
    let error = invalid_project(root.path(), r#"{"maxInstability":"\u0000matis:nonfinite:NaN","maxParameters":NaN}"#);
    assert!(error.contains("type=float_type"));
    assert!(error.contains("type=int_type, input_value=nan"));
}

#[test]
fn reverse_impact_handles_long_chains_without_recursion() {
    let count = 4096;
    let nodes: Vec<_> = (0..count).map(|i| format!("n{i:04}")).collect();
    let edges: Vec<_> = nodes.windows(2).map(|pair| edge(&pair[0], &pair[1], "DEPENDS_ON")).collect();
    let metrics: BTreeMap<_, _> = dependency_metrics(&json!({"nodes": nodes, "edges": edges}));
    assert_eq!(metrics["n4095"]["transitiveChangeImpact"], 4095);
    assert_eq!(metrics["n0000"]["transitiveChangeImpact"], 0);
}
