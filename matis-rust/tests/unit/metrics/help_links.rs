use super::*;

#[test]
fn every_check_rule_links_to_a_default_guide_section() {
    let report = json!({"nodes": [
        {"kind": "file", "level": "file", "metrics": {"lineCount": 1000, "transitiveChangeImpact": 2, "instability": 1, "fanIn": 31, "fanOut": 21}},
        {"kind": "function", "metrics": {"lineCount": 200, "parameterCount": 4,
            "weightedComplexity": 9, "maxDecisionDepth": 4, "nestedLoops": 1}}
    ], "summary": {"circularImportGroups": 1}, "circularImports": [],
        "diagnostics": [{"path": "broken.rs", "message": "Parse error"}]});
    let check = build_check(&report, &json!({"maxChangeImpact": 0, "maxInstability": 0})).unwrap();
    let guide = include_str!("../../../../docs/check-fixes.md");
    let findings = check["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 12);
    for finding in findings {
        let rule = finding["rule"].as_str().unwrap();
        assert!(guide.contains(&format!("## {rule}\n")), "{rule}");
        let expected =
            format!("https://github.com/LeonFelipeCordero/matis/blob/main/docs/check-fixes.md#{}", rule.to_lowercase());
        assert_eq!(finding["helpUrl"], expected);
        assert!(render_check(&check).contains(&format!("How to fix: <{expected}>")));
    }
}

#[test]
fn custom_help_links_override_only_the_selected_rule() {
    let config = json!({"helpLinks": {"maxParameters": "https://example.com/parameters#team"}});
    assert_eq!(validate_project_config(&config).unwrap()["helpLinks"], config["helpLinks"]);
    assert_eq!(validate_config(&config).unwrap()["helpLinks"], config["helpLinks"]);
    let report = json!({"nodes": [{"kind": "function", "metrics": {"lineCount": 200, "parameterCount": 4,
        "weightedComplexity": 0, "maxDecisionDepth": 0, "nestedLoops": 0}}],
        "summary": {"circularImportGroups": 0}, "diagnostics": []});
    let check = build_check(&report, &config).unwrap();
    assert_eq!(
        check["findings"][0]["helpUrl"],
        "https://github.com/LeonFelipeCordero/matis/blob/main/docs/check-fixes.md#maxfunctionlines"
    );
    assert_eq!(check["findings"][1]["helpUrl"], "https://example.com/parameters#team");
    assert!(render_check(&check).contains("How to fix: <https://example.com/parameters#team>"));
    assert_eq!(
        build_check(&json!({"nodes": [], "diagnostics": [], "summary": {"circularImportGroups": 0}}), &config).unwrap(),
        json!({"findings": []})
    );
}

#[test]
fn fan_help_links_support_overrides_and_respect_exclusions() {
    let report = json!({"nodes": [{"kind": "file", "level": "file", "path": "shared.rs",
        "metrics": {"lineCount": 1, "fanIn": 31, "fanOut": 21}}],
        "summary": {"circularImportGroups": 0}, "diagnostics": []});
    let mut config = json!({"helpLinks": {
        "maxFanIn": "https://example.com/fan-in", "maxFanOut": "https://example.com/fan-out"}});
    assert_eq!(validate_project_config(&config).unwrap()["helpLinks"], config["helpLinks"]);
    let check = build_check(&report, &config).unwrap();
    let findings = check["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 2);
    for finding in findings {
        let rule = finding["rule"].as_str().unwrap();
        assert_eq!(finding["helpUrl"], config["helpLinks"][rule]);
        assert!(render_check(&check).contains(finding["helpUrl"].as_str().unwrap()));
    }
    config["fanExclude"] = json!(["shared.rs"]);
    assert_eq!(build_check(&report, &config).unwrap()["findings"], json!([]));
}

#[test]
fn help_links_reject_unknown_rules_and_invalid_values() {
    for links in [
        json!(null),
        json!([]),
        json!({"typo": "https://example.com"}),
        json!({"maxParameters": null}),
        json!({"maxParameters": 12}),
        json!({"maxParameters": ""}),
        json!({"maxParameters": "javascript:alert(1)"}),
        json!({"maxParameters": "https://"}),
        json!({"maxParameters": "https:///guide"}),
        json!({"maxParameters": "https://#guide"}),
        json!({"maxParameters": "https://?guide"}),
        json!({"analysis": "https://example.com/with space"}),
        json!({"analysis": "https://example.com/<script>"}),
        json!({"analysis": "https://example.com/\u{001b}"}),
    ] {
        assert!(validate_config(&json!({"helpLinks": links})).is_err(), "{links}");
        assert!(validate_project_config(&json!({"helpLinks": links})).is_err(), "{links}");
    }
    assert!(validate_config(&json!({"helpLinks": {"analysis": "http://localhost/guide"}})).is_ok());
}
