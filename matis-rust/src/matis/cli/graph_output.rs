use serde_json::{Value, json};

/// Recursively copy navigation JSON and label stored external identities as unresolved.
///
/// Preserve stable keys and the input value. Change CLI labels and ambiguity
/// reasons only; this does not update stored graph facts.
pub fn graph_output(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(graph_output).collect()),
        Value::Object(fields) => {
            project(fields.iter().map(|(key, value)| (key.clone(), graph_output(value))).collect())
        }
        _ => value.clone(),
    }
}
fn project(fields: serde_json::Map<String, Value>) -> Value {
    let mut value = Value::Object(fields);
    if value["level"] == "external" {
        value["level"] = json!("unresolved");
        if value.get("stableKey").is_some() {
            value["type"] = json!("unresolved");
            value["external"] = json!(false);
            value["unresolved"] = json!(true);
            if let Some(name) = value["qualifiedName"].as_str().and_then(|n| n.strip_prefix("external:")) {
                value["qualifiedName"] = json!(format!("unresolved:{name}"));
            }
        }
    }
    if value["reason"] == "external_unresolved_or_ambiguous" {
        value["reason"] = json!("unresolved_or_ambiguous");
    }
    value
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projects_unresolved_identity_without_changing_keys() {
        let original = json!({"nodes": [{"stableKey": "external:x", "level": "external", "qualifiedName": "external:x"}], "level": "external"});
        let result = graph_output(&original);
        assert_eq!(result["nodes"][0]["stableKey"], "external:x");
        assert_eq!(result["nodes"][0]["qualifiedName"], "unresolved:x");
        assert_eq!(result["nodes"][0]["external"], false);
        assert!(result.get("type").is_none());
        assert_eq!(original["level"], "external");
    }
}
