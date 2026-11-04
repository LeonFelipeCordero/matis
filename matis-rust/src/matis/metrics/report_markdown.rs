pub use crate::mutations::report::code;
use crate::mutations::report::render_candidates_markdown;
use serde_json::Value;

pub const METRIC_LABELS: [(&str, &str); 14] = [
    ("lineCount", "Lines"),
    ("parameterCount", "Parameters"),
    ("cyclomaticComplexity", "Cyclomatic complexity"),
    ("weightedComplexity", "Weighted complexity"),
    ("nestedDecisionPenalty", "Nested decision penalty"),
    ("nestedLoops", "Nested loops"),
    ("maxDecisionDepth", "Decision depth"),
    ("fanIn", "Fan-in"),
    ("fanOut", "Fan-out"),
    ("transitiveChangeImpact", "Change impact"),
    ("instability", "Internal instability"),
    ("functionCount", "Functions/methods"),
    ("measuredFunctionCount", "Measured functions/methods"),
    ("weightedComplexitySum", "Weighted complexity sum"),
];
/// Return the display label for a metric key, or `name` for unknown keys.
pub fn metric_label(name: &str) -> &str {
    METRIC_LABELS.iter().find_map(|(key, label)| (*key == name).then_some(*label)).unwrap_or(name)
}

/// Format a report node's path with its optional line, using `Unknown file` for a missing path.
pub fn location(node: &Value) -> String {
    let path = node["path"].as_str().filter(|path| !path.is_empty()).unwrap_or("Unknown file");
    if !node["line"].is_null() { format!("{path}:{}", node["line"]) } else { path.to_owned() }
}

/// Format report values: null as `N/A`, floats to two decimals, and booleans as Python text.
/// Return strings unchanged and serialize other values as JSON.
pub fn value_text(value: &Value) -> String {
    match value {
        Value::Null => "N/A".into(),
        Value::Number(number) if number.is_f64() => format!("{:.2}", number.as_f64().unwrap()),
        Value::String(text) => text.clone(),
        Value::Bool(value) => if *value { "True" } else { "False" }.into(),
        _ => value.to_string(),
    }
}

fn table(title: &str, nodes: &[&Value], names: &[&str]) -> String {
    let labels: Vec<_> = ["Item", "Location"].into_iter().chain(names.iter().map(|name| metric_label(name))).collect();
    let mut rows = vec![
        format!("## {title}"),
        String::new(),
        format!("| {} |", labels.join(" | ")),
        format!("| {} |", vec!["---"; labels.len()].join(" | ")),
    ];
    for node in nodes {
        let cells: Vec<_> = [code(node["subject"].as_str().unwrap_or("")), code(&location(node))]
            .into_iter()
            .chain(names.iter().map(|name| value_text(&node["metrics"][name])))
            .collect();
        rows.push(format!("| {} |", cells.join(" | ")));
    }
    rows.join("\n")
}

fn size_tables(nodes: &[Value]) -> Vec<String> {
    let files: Vec<_> = nodes.iter().filter(|node| node["kind"] == "file").collect();
    let callables: Vec<_> =
        nodes.iter().filter(|node| node["kind"] == "function" || node["kind"] == "method").collect();
    let mut parts = Vec::new();
    if !files.is_empty() {
        parts.push(table(
            "Files",
            &files,
            &["lineCount", "functionCount", "measuredFunctionCount", "weightedComplexitySum"],
        ));
    }
    if !callables.is_empty() {
        parts.push(table(
            "Callables",
            &callables,
            &[
                "lineCount",
                "parameterCount",
                "cyclomaticComplexity",
                "weightedComplexity",
                "nestedDecisionPenalty",
                "nestedLoops",
                "maxDecisionDepth",
            ],
        ));
    }
    parts
}

fn dependencies(nodes: &[Value]) -> Vec<String> {
    if nodes.is_empty() {
        return vec![];
    }
    let nodes: Vec<_> = nodes.iter().filter(|node| node["metrics"].get("fanIn").is_some()).collect();
    vec![table("Dependencies", &nodes, &["fanIn", "fanOut", "transitiveChangeImpact", "instability"]),
        "Dependency metrics count distinct internal peers at each item's level, including tests and type-only dependencies. Impact excludes the item itself. Instability = fan-out / (fan-in + fan-out); isolated items are N/A.".into()]
}

fn cycles(groups: &[Value]) -> String {
    let mut rows = vec!["## Circular imports".into(), String::new()];
    for (index, group) in groups.iter().enumerate() {
        rows.extend([
            format!("### Group {}", index + 1),
            String::new(),
            format!("Files: {}. Internal dependencies: {}.", group["groupSize"], group["internalDependencyCount"]),
        ]);
        rows.extend(array(group, "files").iter().map(|path| format!("- {}", code(path.as_str().unwrap_or("")))));
        rows.push(String::new());
        rows.push(format!(
            "Example cycle: {}",
            array(group, "exampleCycle")
                .iter()
                .map(|path| code(path.as_str().unwrap_or("")))
                .collect::<Vec<_>>()
                .join(" → ")
        ));
        rows.push(String::new());
    }
    rows.join("\n").trim_end().to_owned()
}

fn array<'a>(value: &'a Value, field: &str) -> &'a [Value] {
    value[field].as_array().map(Vec::as_slice).unwrap_or_default()
}

/// Render summary, measurement tables, cycles, diagnostics, and mutation candidates as Markdown.
/// Missing or non-array report sections are treated as empty; no output is written to disk.
pub fn render_markdown(report: &Value) -> String {
    let summary = report["summary"]
        .as_object()
        .map(|summary| summary.iter().map(|(name, value)| format!("- {name}: {value}")).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default();
    let mut parts = vec!["# Metrics".into(), summary];
    parts.extend(size_tables(array(report, "nodes")));
    parts.extend(dependencies(array(report, "nodes")));
    if !array(report, "circularImports").is_empty() {
        parts.push(cycles(array(report, "circularImports")));
    }
    if !array(report, "diagnostics").is_empty() {
        let diagnostics = array(report, "diagnostics")
            .iter()
            .map(|diagnostic| {
                format!(
                    "- {}: {}",
                    code(diagnostic["path"].as_str().filter(|s| !s.is_empty()).unwrap_or("Unknown file")),
                    diagnostic["message"].as_str().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        parts.push(format!("## Analysis diagnostics\n\n{diagnostics}"));
    }
    let candidates = render_candidates_markdown(report);
    if !candidates.is_empty() {
        parts.push(candidates);
    }
    parts.join("\n\n")
}
