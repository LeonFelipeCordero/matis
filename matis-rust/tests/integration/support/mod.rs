//! Shared sample, snapshot, process, and database helpers for integration tests.
pub mod cli;
pub mod project;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/code_samples").canonicalize().unwrap()
}

pub fn snapshot(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/snapshots").join(relative);
    serde_json::from_slice(&fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))).unwrap()
}

#[allow(dead_code)] // Only artifact and navigation parity need the complete production graph.
pub fn assert_artifact(language: &str, graph: &Value) {
    let root = fixtures().join(language);
    assert_eq!(graph["repository"]["path"], root.to_str().unwrap());
    let indexed_at = graph["repository"]["indexedAt"].as_str().expect("indexedAt string");
    chrono::DateTime::parse_from_rfc3339(indexed_at).expect("timezone-aware indexedAt");
    assert_eq!(graph["nodes"][0]["properties"]["absolutePath"], root.to_str().unwrap());
    let mut actual = graph.clone();
    actual["repository"].as_object_mut().unwrap().remove("indexedAt");
    let canonical = format!("/samples/coffee/{language}");
    actual["repository"]["path"] = json!(canonical);
    actual["nodes"][0]["properties"]["absolutePath"] = json!(canonical);
    assert_json(&actual, &snapshot(&format!("coffee/{language}/graph.json")), "");
}

pub fn assert_json(actual: &Value, expected: &Value, pointer: &str) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            assert_eq!(
                actual.keys().collect::<BTreeSet<_>>(),
                expected.keys().collect::<BTreeSet<_>>(),
                "keys at JSON pointer {pointer:?}"
            );
            for (key, value) in expected {
                let escaped = key.replace('~', "~0").replace('/', "~1");
                assert_json(&actual[key], value, &format!("{pointer}/{escaped}"));
            }
        }
        (Value::Array(actual), Value::Array(expected)) => {
            assert_eq!(actual.len(), expected.len(), "array length at JSON pointer {pointer:?}");
            for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                assert_json(actual, expected, &format!("{pointer}/{index}"));
            }
        }
        _ => assert_eq!(actual, expected, "value at JSON pointer {pointer:?}"),
    }
}
