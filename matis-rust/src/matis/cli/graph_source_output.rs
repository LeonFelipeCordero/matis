use super::options::json_dump;
use serde_json::{Value, json};

/// Render a single or batch source response as JSON metadata and numbered excerpts.
///
/// Separate sections with blank lines without reading files or changing `payload`.
///
/// # Panics
/// Panics if the payload is not an object, a single response lacks an object
/// `source`, or an excerpt is not an object.
pub fn source_text(payload: &Value) -> String {
    let mut metadata = payload.clone();
    metadata.as_object_mut().unwrap().shift_remove("source");
    metadata.as_object_mut().unwrap().shift_remove("sources");
    let mut sections = vec![json_dump(&metadata)];
    let excerpts = payload["sources"].as_array().cloned().unwrap_or_else(|| {
        let mut value = json!({"stableKey": payload["filters"]["key"]});
        value.as_object_mut().unwrap().extend(payload["source"].as_object().unwrap().clone());
        vec![value]
    });
    sections.extend(excerpts.iter().map(excerpt_text));
    sections.join("\n\n")
}
/// Render excerpt metadata and source lines with absolute one-based numbers.
///
/// Use `path`, or `stableKey` if absent, as the heading; default `startLine` to 1.
/// Omit `lines` from the metadata and emit no trailing newline.
///
/// # Panics
/// Panics if `excerpt` is not an object.
pub fn excerpt_text(excerpt: &Value) -> String {
    let mut details = excerpt.clone();
    details.as_object_mut().unwrap().shift_remove("lines");
    let name = excerpt["path"].as_str().or_else(|| excerpt["stableKey"].as_str()).unwrap_or("");
    let mut lines = vec![format!("### {name}"), json_dump(&details)];
    let start = excerpt["startLine"].as_u64().unwrap_or(1);
    if let Some(source) = excerpt["lines"].as_array() {
        lines.extend(
            source.iter().enumerate().map(|(i, line)| format!("{}: {}", start + i as u64, line.as_str().unwrap_or(""))),
        );
    }
    lines.join("\n")
}
