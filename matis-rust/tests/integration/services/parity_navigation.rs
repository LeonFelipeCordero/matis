//! Replay all canonical navigation requests against freshly indexed native SQLite graphs.
//! Goldens supply request filters only; no golden graph or response populates storage.
use crate::support;
use matis::{
    bootstrap::{migrate_database, navigation_service, repository_service},
    cli::graph_output::graph_output,
    domain::navigation::{PathRequest, TraversalRequest},
    indexer::analyze_repository,
    services::navigation::NavigationService,
};
use serde_json::{Map, Value, json};
use std::path::Path;
use support::{assert_artifact, assert_json, fixtures, snapshot};

fn traversal(filters: &Value, path: bool) -> TraversalRequest {
    TraversalRequest {
        key: filters[if path { "from" } else { "key" }].as_str().unwrap().to_owned(),
        direction: filters["direction"].as_str().unwrap().to_owned(),
        relations: serde_json::from_value(filters["relations"].clone()).unwrap(),
        depth: filters[if path { "maxDepth" } else { "depth" }].as_u64().unwrap().try_into().unwrap(),
        max_nodes: filters["maxNodes"].as_u64().unwrap().try_into().unwrap(),
        max_edges: filters["maxEdges"].as_u64().unwrap().try_into().unwrap(),
    }
}

fn traversal_or_source(service: &mut NavigationService, command: &str, filters: &Value) -> Value {
    match command {
        "context" => service.context(&traversal(filters, false)).unwrap(),
        "path" => service
            .path(&PathRequest {
                target: filters["to"].as_str().unwrap().to_owned(),
                traversal: traversal(filters, true),
            })
            .unwrap(),
        "source" => service
            .source(
                filters["key"].as_str().unwrap(),
                filters["maxLines"].as_u64().unwrap().try_into().unwrap(),
                filters["startLine"].as_u64().map(Into::into),
            )
            .unwrap(),
        _ => panic!("Uncovered navigation request command: {command}"),
    }
}

fn query(service: &mut NavigationService, label: &str, filters: &Value) -> Value {
    let command = label.split('_').next().unwrap();
    let result = match command {
        "overview" => service.overview().unwrap(),
        "find" => service
            .find(filters["query"].as_str().unwrap(), filters["limit"].as_u64().unwrap().try_into().unwrap())
            .unwrap(),
        "show" => service.show(filters["key"].as_str().unwrap()).unwrap(),
        "children" => service
            .children(
                filters["key"].as_str().unwrap(),
                filters["limit"].as_u64().unwrap().try_into().unwrap(),
                filters["offset"].as_u64().unwrap().into(),
            )
            .unwrap(),
        _ => traversal_or_source(service, command, filters),
    };
    graph_output(&result)
}

fn root_string(value: &str, root: &str, replacement: &str) -> String {
    if value == root || value.starts_with(&format!("{root}/")) {
        format!("{replacement}{}", &value[root.len()..])
    } else {
        value.to_owned()
    }
}

fn normalize_root(value: &Value, root: &str, replacement: &str) -> Value {
    match value {
        Value::Object(fields) => fields
            .iter()
            .map(|(key, value)| (key.clone(), normalize_root(value, root, replacement)))
            .collect::<Map<_, _>>()
            .into(),
        Value::Array(items) => items.iter().map(|item| normalize_root(item, root, replacement)).collect(),
        Value::String(value) => json!(root_string(value, root, replacement)),
        _ => value.clone(),
    }
}

fn normalize_response(response: &Value, root: &Path, metadata: &Value) -> Value {
    let mut response = response.clone();
    for (key, placeholder) in
        [("repositoryId", "<repository-id>"), ("graphRevision", "<graph-revision>"), ("indexedAt", "<indexed-at>")]
    {
        assert!(metadata[key].as_str().is_some_and(|value| !value.is_empty()), "nonempty metadata {key}");
        assert_json(&response[key], &metadata[key], &format!("/{key}"));
        response[key] = json!(placeholder);
    }
    let replacement = format!("/samples/coffee/{}", root.file_name().unwrap().to_str().unwrap());
    normalize_root(&response, root.to_str().unwrap(), &replacement)
}

fn navigation_snapshot(language: &str) {
    let root = fixtures().join(language);
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("matis.sqlite");
    migrate_database(Some(&database)).unwrap();
    let graph = analyze_repository(&root).unwrap();
    assert_artifact(language, &graph);
    let saved = repository_service(Some(&database)).unwrap().save_folder(&root, &graph).unwrap();
    let metadata = json!({"repositoryId": saved.repository_id, "graphRevision": saved.graph_revision, "indexedAt": saved.indexed_at});
    let mut service = navigation_service(Some(&saved.repository_id), None, Some(&database)).unwrap();
    let expected = snapshot(&format!("coffee/{language}/navigation.json"));
    let actual: Map<_, _> = expected
        .as_object()
        .unwrap()
        .iter()
        .map(|(label, response)| {
            let actual = query(&mut service, label, &response["filters"]);
            (label.clone(), normalize_response(&actual, &root, &metadata))
        })
        .collect();
    assert_json(&actual.into(), &expected, "");
}

macro_rules! navigation_tests {
    ($($language:ident),+ $(,)?) => {$ (
        #[test] fn $language() { navigation_snapshot(stringify!($language)); }
    )+};
}
navigation_tests!(java, kotlin, go, rust, javascript, typescript, elixir, python);

#[test]
fn normalization_preserves_graph_content_and_array_order() {
    let metadata = json!({"repositoryId":"repo", "graphRevision":"revision", "indexedAt":"timestamp"});
    let response = json!({"repositoryId":"repo", "graphRevision":"revision", "indexedAt":"timestamp",
        "root":"/checkout/python", "absolutePath":"/checkout/python/report.py", "nodes":[{"stableKey":"z"},{"stableKey":"a"}],
        "edges":[{"evidence":"render()", "confidence":"INFERRED", "ordinal":7}], "source":{"lines":["def render():","    return 1"]}});
    let mut expected = response.clone();
    expected["repositoryId"] = json!("<repository-id>");
    expected["graphRevision"] = json!("<graph-revision>");
    expected["indexedAt"] = json!("<indexed-at>");
    expected["root"] = json!("/samples/coffee/python");
    expected["absolutePath"] = json!("/samples/coffee/python/report.py");
    assert_json(&normalize_response(&response, Path::new("/checkout/python"), &metadata), &expected, "");
    assert_eq!(response["graphRevision"], "revision");
}

#[test]
fn normalization_rejects_inconsistent_metadata() {
    let metadata = json!({"repositoryId":"repo", "graphRevision":"revision", "indexedAt":"timestamp"});
    for field in ["repositoryId", "graphRevision", "indexedAt"] {
        let mut response = metadata.clone();
        response[field] = json!("different");
        assert!(
            std::panic::catch_unwind(|| normalize_response(&response, Path::new("/checkout/python"), &metadata))
                .is_err()
        );
    }
}
