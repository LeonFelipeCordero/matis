use super::models::*;
use serde_json::{Value, json};

/// Convert a node and optional symbol row to navigation JSON, using null for absent symbol fields.
pub fn node_value(node: GraphNodeRow, symbol: Option<GraphSymbolRow>) -> Value {
    let mut value = json!({"stableKey": node.node_key, "parentStableKey": node.parent_key,
        "type": node.type_, "kind": node.kind, "level": node.level, "name": node.name,
        "language": node.language, "qualifiedName": node.qualified_name, "path": node.path,
        "startLine": null, "startColumn": null, "endLine": null, "endColumn": null,
        "signature": null, "visibility": null});
    if let Some(s) = symbol {
        value["startLine"] = json!(s.start_line);
        value["startColumn"] = json!(s.start_column);
        value["endLine"] = json!(s.end_line);
        value["endColumn"] = json!(s.end_column);
        value["signature"] = json!(s.signature);
        value["visibility"] = json!(s.visibility);
    }
    value
}
/// Convert stored metrics to camelCase navigation fields, preserving null values.
pub fn metric_value(m: GraphNodeMetricRow) -> Value {
    json!({"lineCount": m.line_count, "parameterCount": m.parameter_count,
        "baseCyclomaticComplexity": m.base_complexity, "nestedCyclomaticComplexity": m.nested_complexity,
        "loopNestedComplexity": m.loop_complexity, "cyclomaticComplexity": m.total_complexity,
        "maxDecisionDepth": m.max_decision_depth})
}
/// Convert an edge row to navigation JSON; include its artifact ordinal only when `ordinal` is true.
pub fn edge_value(e: GraphEdgeRow, ordinal: bool) -> Value {
    let mut value = json!({"sourceStableKey": e.source_key, "targetStableKey": e.target_key,
        "type": e.relationship, "confidence": e.confidence, "path": e.path,
        "evidence": e.evidence, "line": e.line, "typeOnly": e.type_only});
    if ordinal {
        value.as_object_mut().unwrap().insert("ordinal".into(), json!(e.ordinal));
    }
    value
}
/// Convert a diagnostic row to severity, path, language, and message fields without storage identifiers.
pub fn diagnostic_value(d: GraphDiagnosticRow) -> Value {
    json!({"severity": d.severity, "path": d.path, "language": d.language, "message": d.message})
}
