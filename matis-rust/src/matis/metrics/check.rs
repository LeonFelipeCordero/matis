use super::{
    config::{integer_text, validate_config},
    report_markdown::{code, location, metric_label, value_text},
};
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::{cmp::Ordering, collections::BTreeSet};

const RULES: [(&str, &str, &[&str]); 6] = [
    ("maxFileLines", "lineCount", &["file"]),
    ("maxFunctionLines", "lineCount", &["function", "method"]),
    ("maxParameters", "parameterCount", &["function", "method"]),
    ("maxWeightedComplexity", "weightedComplexity", &["function", "method"]),
    ("maxDecisionDepth", "maxDecisionDepth", &["function", "method"]),
    ("maxNestedLoops", "nestedLoops", &["function", "method"]),
];
const DEPENDENCY_RULES: [(&str, &str); 2] =
    [("maxChangeImpact", "transitiveChangeImpact"), ("maxInstability", "instability")];

fn metric_finding(node: &Value, limit: (&str, &str, &Value)) -> Result<Option<Value>> {
    let (rule, metric, maximum) = limit;
    if maximum.is_null() {
        return Ok(None);
    }
    let metrics = node["metrics"].as_object().ok_or_else(|| anyhow!("metrics must be an object"))?;
    if metric == "instability" && metrics.get("fanIn") == Some(&json!(0)) && metrics.get("fanOut") == Some(&json!(0)) {
        return Ok(None);
    }
    let actual = metrics.get(metric).unwrap_or(&Value::Null);
    if !actual.is_null() && within_limit(actual, maximum, metric)? {
        return Ok(None);
    }
    let label = metric_label(metric);
    let details = if actual.is_null() {
        format!("{label} is unavailable; cannot evaluate maximum {maximum}.")
    } else {
        format!("{label} is {actual} (maximum: {maximum}).")
    };
    Ok(Some(json!({"rule": rule, "metric": metric, "subject": node["subject"],
        "files": [{"path": node["path"], "line": node["line"]}], "actual": actual, "limit": maximum, "details": [details]})))
}

fn within_limit(actual: &Value, maximum: &Value, metric: &str) -> Result<bool> {
    match (integer_text(actual), integer_text(maximum)) {
        (Some(actual), Some(maximum)) => Ok(compare_integers(actual, maximum) != Ordering::Greater),
        (None, Some(maximum)) => {
            Ok(compare_float_integer(numeric_value(actual, metric)?, maximum) != Ordering::Greater)
        }
        (Some(actual), None) => Ok(compare_float_integer(numeric_value(maximum, metric)?, actual) != Ordering::Less),
        (None, None) => Ok(numeric_value(actual, metric)? <= numeric_value(maximum, metric)?),
    }
}
fn numeric_value(value: &Value, metric: &str) -> Result<f64> {
    value.as_f64().ok_or_else(|| anyhow!("{metric} must be numeric"))
}
fn compare_float_integer(value: f64, integer: &str) -> Ordering {
    let floor = format!("{:.0}", value.floor());
    let order = compare_integers(&floor, integer);
    if order == Ordering::Equal && value.fract() != 0.0 { Ordering::Greater } else { order }
}
fn compare_integers(left: &str, right: &str) -> Ordering {
    let (left_negative, left) = integer_parts(left);
    let (right_negative, right) = integer_parts(right);
    if left_negative != right_negative {
        return right_negative.cmp(&left_negative);
    }
    let order = left.len().cmp(&right.len()).then_with(|| left.cmp(right));
    if left_negative { order.reverse() } else { order }
}
fn integer_parts(text: &str) -> (bool, &str) {
    let digits = text.trim_start_matches('-').trim_start_matches('0');
    if digits.is_empty() {
        return (false, "0");
    }
    (text.starts_with('-'), digits)
}

fn enabled_limits<'a>(node: &Value, config: &'a Value) -> Vec<(&'static str, &'static str, &'a Value)> {
    let kind = node["kind"].as_str().unwrap_or("");
    let mut rules: Vec<_> = RULES
        .into_iter()
        .filter(|(_, _, kinds)| kinds.contains(&kind))
        .map(|(rule, metric, _)| (rule, metric))
        .collect();
    if node["level"] == "module" || node["level"] == "file" {
        rules.extend(DEPENDENCY_RULES);
    }
    rules.into_iter().map(|(rule, metric)| (rule, metric, &config[rule])).collect()
}
fn node_findings(node: &Value, config: &Value) -> Result<Vec<Value>> {
    let mut findings = Vec::new();
    for limit in enabled_limits(node, config) {
        if let Some(finding) = metric_finding(node, limit)? {
            findings.push(finding);
        }
    }
    Ok(findings)
}

fn cycle_finding(report: &Value, maximum: &Value) -> Result<Option<Value>> {
    if maximum.is_null() {
        return Ok(None);
    }
    let count = &report["summary"]["circularImportGroups"];
    if integer_text(count).is_none() {
        return Err(anyhow!("circularImportGroups must be an integer"));
    }
    if within_limit(count, maximum, "circularImportGroups")? {
        return Ok(None);
    }
    let groups = report["circularImports"].as_array().ok_or_else(|| anyhow!("circularImports must be an array"))?;
    let mut paths = BTreeSet::new();
    let mut details = vec![format!("Circular import groups: {count} (maximum: {maximum}).")];
    for group in groups {
        paths.extend(cycle_paths(group)?);
        let cycle: Vec<_> = group["exampleCycle"]
            .as_array()
            .ok_or_else(|| anyhow!("exampleCycle must be an array"))?
            .iter()
            .map(|path| path.as_str().unwrap_or(""))
            .collect();
        details.push(format!("Example cycle: {}", cycle.join(" → ")));
    }
    let files: Vec<_> = paths.iter().map(|path| json!({"path": path})).collect();
    Ok(Some(json!({"rule": "maxCircularImportGroups", "metric": "circularImportGroups", "subject": "Circular imports",
        "actual": count, "limit": maximum, "files": files, "details": details})))
}

fn cycle_paths(group: &Value) -> Result<Vec<String>> {
    group["files"]
        .as_array()
        .ok_or_else(|| anyhow!("cycle files must be an array"))?
        .iter()
        .map(|path| path.as_str().map(str::to_owned).ok_or_else(|| anyhow!("cycle file must be a string")))
        .collect()
}

/// Evaluate `report` against validated `config` limits and return a findings object.
/// Null limits disable checks; unavailable enabled metrics and parse diagnostics become findings.
///
/// # Errors
/// Reject invalid settings, malformed report arrays, or nonnumeric checked metrics.
pub fn build_check(report: &Value, config: &Value) -> Result<Value> {
    let config = validate_config(config)?;
    let mut findings = Vec::new();
    for node in report["nodes"].as_array().ok_or_else(|| anyhow!("nodes must be an array"))? {
        findings.extend(node_findings(node, &config)?);
    }
    if let Some(finding) = cycle_finding(report, &config["maxCircularImportGroups"])? {
        findings.push(finding);
    }
    for diagnostic in report["diagnostics"].as_array().ok_or_else(|| anyhow!("diagnostics must be an array"))? {
        findings.push(json!({"rule": "analysis", "metric": "parseError", "subject": "Analysis incomplete",
            "files": [{"path": diagnostic["path"]}], "actual": null, "limit": null, "details": [diagnostic["message"]]}));
    }
    Ok(json!({"findings": findings}))
}

fn finding_section(finding: &Value) -> String {
    let mut rows = vec![
        format!("## {}: {}", finding["rule"].as_str().unwrap_or(""), code(finding["subject"].as_str().unwrap_or(""))),
        String::new(),
        format!("Measured: {}; maximum: {}.", value_text(&finding["actual"]), value_text(&finding["limit"])),
        String::new(),
    ];
    for file in finding["files"].as_array().into_iter().flatten() {
        rows.push(format!("- {}", code(&location(file))));
    }
    rows.push(String::new());
    for detail in finding["details"].as_array().into_iter().flatten() {
        rows.push(format!("- {}", detail.as_str().unwrap_or("")));
    }
    rows.join("\n")
}

/// Format check findings as Markdown, or return `All checks passed.` when none exist.
/// Missing or non-array `findings` is treated as empty.
pub fn render_check(check: &Value) -> String {
    let findings = check["findings"].as_array().map(Vec::as_slice).unwrap_or_default();
    if findings.is_empty() {
        return "All checks passed.".into();
    }
    findings.iter().map(finding_section).collect::<Vec<_>>().join("\n\n")
}
