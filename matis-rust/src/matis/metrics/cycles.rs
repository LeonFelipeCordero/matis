use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub type Adjacency = BTreeMap<String, Vec<String>>;

/// Return sorted members of each strongly connected component in source-first order.
/// Traverse `adjacency` iteratively with Kosaraju's algorithm.
///
/// # Panics
/// Panic if an edge target has no entry in `adjacency`.
pub fn strongly_connected_components(adjacency: &Adjacency) -> Vec<Vec<String>> {
    let mut visited = BTreeSet::new();
    let mut finished = Vec::new();
    for root in adjacency.keys() {
        if !visited.insert(root.clone()) {
            continue;
        }
        finish_from(root, adjacency, (&mut visited, &mut finished));
    }
    let reverse = reverse_adjacency(adjacency);
    visited.clear();
    let mut groups = Vec::new();
    for root in finished.into_iter().rev() {
        if !visited.insert(root.clone()) {
            continue;
        }
        groups.push(reverse_group(root, &reverse, &mut visited));
    }
    groups
}

fn reverse_adjacency(adjacency: &Adjacency) -> Adjacency {
    let mut reverse: Adjacency = adjacency.keys().map(|key| (key.clone(), vec![])).collect();
    for (source, targets) in adjacency {
        add_reverse_edges(source, targets, &mut reverse);
    }
    reverse
}
fn add_reverse_edges(source: &str, targets: &[String], reverse: &mut Adjacency) {
    for target in targets {
        reverse.get_mut(target).expect("missing adjacency target").push(source.to_owned());
    }
}

fn finish_from(root: &str, adjacency: &Adjacency, state: (&mut BTreeSet<String>, &mut Vec<String>)) {
    let (visited, finished) = state;
    let mut stack = vec![(root.to_owned(), 0)];
    while let Some((node, index)) = stack.last_mut() {
        if *index == adjacency[node].len() {
            finished.push(node.clone());
            stack.pop();
            continue;
        }
        let target = adjacency[node][*index].clone();
        *index += 1;
        if visited.insert(target.clone()) {
            stack.push((target, 0));
        }
    }
}

fn reverse_group(root: String, reverse: &Adjacency, visited: &mut BTreeSet<String>) -> Vec<String> {
    let mut pending = vec![root];
    let mut group = Vec::new();
    while let Some(node) = pending.pop() {
        enqueue_unvisited(&reverse[&node], visited, &mut pending);
        group.push(node);
    }
    group.sort();
    group
}

fn enqueue_unvisited(targets: &[String], visited: &mut BTreeSet<String>, pending: &mut Vec<String>) {
    for target in targets {
        if visited.insert(target.clone()) {
            pending.push(target.clone());
        }
    }
}

fn file_owner(key: &str, lookup: &BTreeMap<&str, &Value>, owners: &mut BTreeMap<String, Option<String>>) {
    let mut current = key;
    let mut trail = BTreeSet::new();
    let mut owner = None;
    while let Some(node) = lookup.get(current) {
        if let Some(cached) = owners.get(current) {
            owner = cached.clone();
            break;
        }
        if node["type"] == "external" || node["level"] == "external" {
            break;
        }
        if node["type"] == "file" {
            owner = Some(current.to_owned());
            break;
        }
        if !trail.insert(current.to_owned()) {
            break;
        }
        current = node["parentStableKey"].as_str().unwrap_or("");
    }
    owners.insert(key.to_owned(), owner.clone());
    for key in trail {
        owners.insert(key, owner.clone());
    }
}

fn import_edges(nodes: &[Value], edges: &[Value]) -> Vec<Value> {
    let lookup: BTreeMap<_, _> = nodes.iter().map(|node| (node["stableKey"].as_str().unwrap_or(""), node)).collect();
    let mut owners = BTreeMap::new();
    for key in lookup.keys() {
        file_owner(key, &lookup, &mut owners);
    }
    let mut projected: Vec<_> = edges.iter().filter_map(|edge| project_import(edge, &owners)).collect();
    projected.sort_by_cached_key(|edge| python_json(edge, true));
    projected
}

fn project_import(edge: &Value, owners: &BTreeMap<String, Option<String>>) -> Option<Value> {
    if edge["type"] != "IMPORTS" || edge["typeOnly"] == true {
        return None;
    }
    let source = edge["sourceStableKey"].as_str()?;
    let target = edge["targetStableKey"].as_str()?;
    let source_file = owners.get(source)?.as_ref()?;
    let target_file = owners.get(target)?.as_ref()?;
    if source_file == target_file && (source != source_file || target != target_file) {
        return None;
    }
    let mut result = json!({"sourceStableKey": source_file, "targetStableKey": target_file});
    for name in ["path", "line", "evidence", "confidence", "type"] {
        if let Some(value) = edge.get(name) {
            result[name] = value.clone();
        }
    }
    Some(result)
}

fn example_cycle(group: &[String], adjacency: &Adjacency) -> Vec<String> {
    let start = &group[0];
    if group.len() == 1 {
        return vec![start.clone(), start.clone()];
    }
    let members: BTreeSet<_> = group.iter().collect();
    let first = adjacency[start].iter().find(|key| *key != start && members.contains(key)).unwrap();
    let mut search = CycleSearch {
        adjacency,
        members,
        start,
        parents: BTreeMap::from([(first.clone(), None)]),
        pending: VecDeque::from([first.clone()]),
    };
    while let Some(node) = search.pending.pop_front() {
        if search.visit_neighbors(&node) {
            return cycle_path(start, &search.parents);
        }
    }
    unreachable!("Strongly connected component has no return path")
}
struct CycleSearch<'a> {
    adjacency: &'a Adjacency,
    members: BTreeSet<&'a String>,
    start: &'a str,
    parents: BTreeMap<String, Option<String>>,
    pending: VecDeque<String>,
}
impl CycleSearch<'_> {
    fn visit_neighbors(&mut self, node: &str) -> bool {
        for target in &self.adjacency[node] {
            if !self.members.contains(target) || self.parents.contains_key(target) {
                continue;
            }
            self.parents.insert(target.clone(), Some(node.to_owned()));
            if target == self.start {
                return true;
            }
            self.pending.push_back(target.clone());
        }
        false
    }
}

fn cycle_path(start: &str, parents: &BTreeMap<String, Option<String>>) -> Vec<String> {
    let mut path = vec![start.to_owned()];
    let mut current = parents[start].as_ref();
    while let Some(node) = current {
        path.push(node.clone());
        current = parents[node].as_ref();
    }
    path.reverse();
    path.insert(0, start.to_owned());
    path
}

/// Return version-1 runtime import cycle groups projected onto owning files.
/// Exclude external and type-only imports; include direct file self-imports.
/// Each group has a stable hash ID, member keys, an example cycle, and internal edges.
/// `nodes` supplies the parent hierarchy for the relationships in `edges`.
pub fn circular_dependencies(nodes: &[Value], edges: &[Value]) -> Value {
    let projected = import_edges(nodes, edges);
    let mut neighbors: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in &projected {
        let source = edge["sourceStableKey"].as_str().unwrap().to_owned();
        let target = edge["targetStableKey"].as_str().unwrap().to_owned();
        neighbors.entry(source).or_default().insert(target.clone());
        neighbors.entry(target).or_default();
    }
    let adjacency: Adjacency = neighbors.into_iter().map(|(key, values)| (key, values.into_iter().collect())).collect();
    let mut components: Vec<_> = strongly_connected_components(&adjacency)
        .into_iter()
        .filter(|group| group.len() > 1 || adjacency[&group[0]].contains(&group[0]))
        .collect();
    components.sort();
    let mut groups = Vec::new();
    let mut membership = BTreeMap::new();
    for keys in components {
        let digest = format!("{:x}", Sha256::digest(python_json(&json!(keys), false).as_bytes()));
        membership.extend(keys.iter().map(|key| (key.clone(), groups.len())));
        groups.push(json!({"id": format!("import-cycle:{digest}"), "nodeStableKeys": keys,
            "exampleCycle": example_cycle(&keys, &adjacency), "edges": []}));
    }
    for edge in projected {
        let source = membership.get(edge["sourceStableKey"].as_str().unwrap());
        let target = membership.get(edge["targetStableKey"].as_str().unwrap());
        if let Some(index) = source.filter(|_| source == target) {
            groups[*index]["edges"].as_array_mut().unwrap().push(edge);
        }
    }
    json!({"version": 1, "groups": groups})
}

/// Serialize `value` with Python-style ASCII strings and spaced separators.
/// Sort object keys recursively when `sort_keys` is true; otherwise retain map order.
pub(crate) fn python_json(value: &Value, sort_keys: bool) -> String {
    match value {
        Value::String(text) => ascii_string(text),
        Value::Array(values) => {
            format!("[{}]", values.iter().map(|v| python_json(v, sort_keys)).collect::<Vec<_>>().join(", "))
        }
        Value::Object(values) => {
            let mut entries: Vec<_> = values.iter().collect();
            if sort_keys {
                entries.sort_by_key(|(key, _)| *key);
            }
            format!(
                "{{{}}}",
                entries
                    .iter()
                    .map(|(key, v)| format!("{}: {}", ascii_string(key), python_json(v, sort_keys)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        _ => value.to_string(),
    }
}

fn ascii_character(ch: char) -> String {
    ch.encode_utf16(&mut [0; 2]).iter().map(|unit| format!("\\u{unit:04x}")).collect()
}

fn ascii_string(text: &str) -> String {
    let encoded = serde_json::to_string(text).unwrap();
    let mut result = String::new();
    for ch in encoded.chars() {
        if ch.is_ascii() && ch != '\u{7f}' {
            result.push(ch);
            continue;
        }
        result.push_str(&ascii_character(ch));
    }
    result
}
