use super::cycles::{Adjacency, strongly_connected_components};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const METRIC_NAMES: [&str; 3] = ["fanIn", "fanOut", "transitiveChangeImpact"];

/// Return distinct fan-in, fan-out, transitive dependent counts, and instability per view node.
/// Edges run from dependent to dependency. Ignore self-edges and endpoints outside `view`.
/// Impact excludes self; isolated nodes have zero counts and null instability.
/// Instability is fan-out / (fan-in + fan-out). The view is not modified.
pub fn dependency_metrics(view: &Value) -> BTreeMap<String, Value> {
    let mut neighbors: BTreeMap<String, BTreeSet<String>> = view["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|key| key.as_str().map(|key| (key.to_owned(), BTreeSet::new())))
        .collect();
    for edge in view["edges"].as_array().into_iter().flatten() {
        let source = edge["sourceStableKey"].as_str().unwrap_or("");
        let target = edge["targetStableKey"].as_str().unwrap_or("");
        if source != target
            && neighbors.contains_key(target)
            && let Some(targets) = neighbors.get_mut(source)
        {
            targets.insert(target.to_owned());
        }
    }
    let adjacency: Adjacency =
        neighbors.into_iter().map(|(key, targets)| (key, targets.into_iter().collect())).collect();
    let mut fan_in: BTreeMap<&str, usize> = BTreeMap::new();
    for target in adjacency.values().flatten() {
        *fan_in.entry(target).or_default() += 1;
    }
    let impact = change_impact(&adjacency);
    adjacency
        .iter()
        .map(|(key, targets)| {
            let incoming = fan_in.get(key.as_str()).copied().unwrap_or(0);
            let outgoing = targets.len();
            let instability = if incoming + outgoing == 0 {
                Value::Null
            } else {
                json!(outgoing as f64 / (incoming + outgoing) as f64)
            };
            (
                key.clone(),
                json!({"fanIn": incoming, "fanOut": outgoing,
            "transitiveChangeImpact": impact[key], "instability": instability}),
            )
        })
        .collect()
}

fn change_impact(adjacency: &Adjacency) -> BTreeMap<String, usize> {
    let groups = strongly_connected_components(adjacency);
    let membership: BTreeMap<_, _> =
        groups.iter().enumerate().flat_map(|(index, group)| group.iter().map(move |key| (key, index))).collect();
    let mut pending: BTreeMap<usize, Vec<u64>> = BTreeMap::new();
    let mut impact = BTreeMap::new();
    let mut offset = 0;
    for (index, group) in groups.iter().enumerate() {
        let mut reachable = pending.remove(&index).unwrap_or_default();
        let count = reachable.iter().map(|bits| bits.count_ones() as usize).sum::<usize>() + group.len() - 1;
        impact.extend(group.iter().map(|key| (key.clone(), count)));
        let targets: BTreeSet<_> = group
            .iter()
            .flat_map(|source| &adjacency[source])
            .map(|target| membership[target])
            .filter(|target| *target != index)
            .collect();
        if !targets.is_empty() {
            // Only components with outgoing edges need bits for downstream impact counts.
            reachable.resize((offset + group.len()).div_ceil(64), 0);
            mark_members(&mut reachable, offset..offset + group.len());
            offset += group.len();
            propagate(&reachable, targets, &mut pending);
        }
    }
    impact
}

fn mark_members(reachable: &mut [u64], members: std::ops::Range<usize>) {
    for bit in members {
        reachable[bit / 64] |= 1 << (bit % 64);
    }
}
fn propagate(reachable: &[u64], targets: BTreeSet<usize>, pending: &mut BTreeMap<usize, Vec<u64>>) {
    for target in targets {
        merge_bits(pending.entry(target).or_default(), reachable);
    }
}

fn merge_bits(target: &mut Vec<u64>, source: &[u64]) {
    target.resize(target.len().max(source.len()), 0);
    for (target, source) in target.iter_mut().zip(source) {
        *target |= source;
    }
}
