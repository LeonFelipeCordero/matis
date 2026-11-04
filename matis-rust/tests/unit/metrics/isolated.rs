//! Core metrics contracts use in-memory records and JSON, not repositories or storage.
use super::*;

#[test]
fn config_line_limits_use_defaults_and_preserve_overrides() {
    let config = validate_config(&json!({})).unwrap();
    assert_eq!(config["maxFileLines"], 700);
    assert_eq!(config["maxFunctionLines"], 150);
    let config = validate_config(&json!({"maxFileLines": 500, "maxFunctionLines": 80})).unwrap();
    assert_eq!(config["maxFileLines"], 500);
    assert_eq!(config["maxFunctionLines"], 80);
}

#[test]
fn config_validation_rejects_coercion_without_reading_a_project() {
    for value in [json!(-1), json!(true), json!("2"), json!(2.5)] {
        assert!(validate_config(&json!({"maxFileLines": value})).is_err(), "{value}");
    }
    assert!(validate_project_config(&json!({"mutations": {"rules": ["unknown"]}})).is_err());
    assert!(validate_project_config(&json!({"unexpected": 1})).is_err());
}

#[test]
fn files_without_callables_skip_grammar_loading() {
    // An unsupported grammar makes an unnecessary parse fail instead of silently slowing this path.
    let mut file = file("data.unknown", "not-a-grammar", b"not syntax");
    callable_metrics(&mut file).unwrap();
    let mut symbol = parsed_symbol(parse("python", b"class Brew: pass").root_node(), "class");
    symbol.line_count = Some(42);
    file.symbols.push(symbol.clone());
    callable_metrics(&mut file).unwrap();
    assert_eq!(serde_json::to_value(&file.symbols).unwrap(), json!([symbol]));
    assert_eq!(file.source.source, b"not syntax");
}

fn reverse_walk_impact(index: usize, links: &[(usize, usize)]) -> usize {
    let mut dependents = std::collections::BTreeSet::from([index]);
    let mut pending = vec![index];
    while let Some(target) = pending.pop() {
        links.iter().for_each(|&(source, dependency)| {
            if dependency == target && dependents.insert(source) {
                pending.push(source);
            }
        });
    }
    dependents.len() - 1
}

fn assert_dependency_metrics_match_walk(nodes: Vec<String>, links: Vec<(usize, usize)>) {
    let edges: Vec<_> =
        links.iter().map(|&(source, target)| edge(&nodes[source], &nodes[target], "DEPENDS_ON")).collect();
    let result = dependency_metrics(&json!({"nodes": nodes, "edges": edges}));
    for (index, key) in nodes.iter().enumerate() {
        let incoming = links
            .iter()
            .filter(|&&(s, t)| s != t && t == index)
            .map(|&(s, _)| s)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let outgoing = links
            .iter()
            .filter(|&&(s, t)| s != t && s == index)
            .map(|&(_, t)| t)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        let instability = if incoming + outgoing == 0 {
            serde_json::Value::Null
        } else {
            json!(outgoing as f64 / (incoming + outgoing) as f64)
        };
        assert_eq!(
            result[key],
            json!({"fanIn": incoming, "fanOut": outgoing,
            "transitiveChangeImpact": reverse_walk_impact(index, &links), "instability": instability}),
            "{key}"
        );
    }
}

#[test]
fn dependency_metrics_match_reverse_walk_for_every_three_node_graph() {
    for mask in 0..512 {
        let links = (0..9).filter(|bit| mask & (1 << bit) != 0).map(|bit| (bit / 3, bit % 3)).collect();
        assert_dependency_metrics_match_walk((0..3).map(|i| i.to_string()).collect(), links);
    }
}

#[test]
fn dependency_metrics_preserve_large_components_isolates_and_diamond_merges() {
    let mut links: Vec<_> = (0..70).map(|i| (i, (i + 1) % 70)).collect();
    links.extend((132..160).map(|i| (i, if i == 159 { 132 } else { i + 1 })));
    links.extend([(0, 130), (0, 131), (130, 132), (131, 132), (131, 132)]);
    assert_dependency_metrics_match_walk((0..160).map(|i| format!("{i:03}")).collect(), links);
}

#[test]
fn dependency_metrics_ignore_unknown_endpoints_and_keep_isolated_nodes() {
    let view = json!({"nodes": ["a", "b", "isolated"], "edges": [
        edge("a", "b", "DEPENDS_ON"),
        edge("a", "missing", "DEPENDS_ON"),
        edge("missing", "b", "DEPENDS_ON"),
        edge("a", "a", "DEPENDS_ON")
    ]});
    let metrics = dependency_metrics(&view);
    assert_eq!(metrics.len(), 3);
    assert_eq!(metrics["a"], json!({"fanIn": 0, "fanOut": 1, "transitiveChangeImpact": 0, "instability": 1.0}));
    assert_eq!(metrics["b"], json!({"fanIn": 1, "fanOut": 0, "transitiveChangeImpact": 1, "instability": 0.0}));
    assert_eq!(metrics["isolated"], json!({"fanIn": 0, "fanOut": 0, "transitiveChangeImpact": 0, "instability": null}));
}
