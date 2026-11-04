use crate::domain::navigation::{EdgeRequest, NavigationReader, TraversalRequest};
use anyhow::Result;
use serde_json::{Map, Value, json};
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct Walk {
    pub nodes: Map<String, Value>,
    pub edges: Map<String, Value>,
    pub parents: HashMap<String, (String, String)>,
    pub cutoffs: Map<String, Value>,
}
impl Walk {
    /// Return visited nodes/edges and expandable cutoffs or non-expandable external boundaries.
    /// Only traversal cutoffs set `truncated`; external nodes alone do not.
    pub fn output(&self) -> Value {
        let mut external: Vec<_> = self.nodes.iter().filter(|(_, n)| n["external"] == true).map(|(k, _)| k).collect();
        external.sort();
        let boundaries: Vec<_> = self
            .cutoffs
            .iter()
            .map(|(k, reason)| json!({"stableKey": k, "reason": reason, "expandable": true}))
            .chain(external.into_iter().map(
                |key| json!({"stableKey": key, "reason": "external_unresolved_or_ambiguous", "expandable": false}),
            ))
            .collect();
        json!({"nodes": self.nodes.values().collect::<Vec<_>>(), "edges": self.edges.values().collect::<Vec<_>>(),
            "boundaries": boundaries, "truncated": !self.cutoffs.is_empty()})
    }
}
struct Traversal<'a> {
    reader: &'a mut dyn NavigationReader,
    request: &'a TraversalRequest,
    walk: Walk,
    frontier: VecDeque<(String, usize)>,
}
impl Traversal<'_> {
    fn run(mut self) -> Result<Walk> {
        while let Some((current, distance)) = self.frontier.pop_front() {
            if self.walk.nodes[&current]["external"] == true {
                continue;
            }
            if distance >= self.request.depth {
                self.walk.cutoffs.insert(current, json!("depth"));
                continue;
            }
            self.expand(&current, distance)?;
        }
        Ok(self.walk)
    }
    fn expand(&mut self, key: &str, distance: usize) -> Result<()> {
        let rows = self.reader.edges(&EdgeRequest {
            key: key.into(),
            direction: self.request.direction.clone(),
            relations: self.request.relations.clone(),
            // Already visited edges can occupy the first max_edges rows.
            limit: self.request.max_edges.saturating_add(1),
        })?;
        for edge in &rows {
            let ordinal = edge["ordinal"].to_string();
            if self.walk.edges.contains_key(&ordinal) {
                continue;
            }
            if self.walk.edges.len() >= self.request.max_edges {
                self.walk.cutoffs.insert(key.into(), json!("maxEdges"));
                break;
            }
            self.add_edge((key, distance), edge.clone())?;
        }
        Ok(())
    }
    fn add_edge(&mut self, position: (&str, usize), edge: Value) -> Result<()> {
        let (key, distance) = position;
        let neighbor = if edge["sourceStableKey"] == key { &edge["targetStableKey"] } else { &edge["sourceStableKey"] }
            .as_str()
            .unwrap()
            .to_owned();
        let ordinal = edge["ordinal"].to_string();
        if !self.walk.nodes.contains_key(&neighbor) {
            if self.walk.nodes.len() >= self.request.max_nodes {
                self.walk.cutoffs.insert(key.into(), json!("maxNodes"));
                return Ok(());
            }
            self.walk.nodes.insert(neighbor.clone(), self.reader.node(&neighbor)?);
            self.walk.parents.insert(neighbor.clone(), (key.into(), ordinal.clone()));
            self.frontier.push_back((neighbor, distance + 1));
        }
        self.walk.edges.insert(ordinal, edge);
        Ok(())
    }
}
/// Return breadth-first state using `request` direction, relations, depth, and budgets.
/// Always include the starting node; do not expand external nodes. Record budget/depth
/// cutoffs and first-discovery parents; propagate missing-node and reader errors.
pub fn traverse(reader: &mut dyn NavigationReader, request: &TraversalRequest) -> Result<Walk> {
    let mut walk = Walk::default();
    walk.nodes.insert(request.key.clone(), reader.node(&request.key)?);
    Traversal { reader, request, walk, frontier: VecDeque::from([(request.key.clone(), 0)]) }.run()
}
/// Reconstruct a discovered route from a `walk` rooted at `start` to `target`.
/// Return ordered keys/edges with `found`, or empty paths with `unknown` after cutoffs
/// and `not_found` after exhaustion. Require consistent parent/edge maps or panic.
pub fn route(walk: &Walk, start: &str, target: &str) -> Value {
    if !walk.nodes.contains_key(target) {
        return json!({"status": if walk.cutoffs.is_empty() { "not_found" } else { "unknown" }, "path": [], "pathEdges": []});
    }
    let mut keys = vec![target.to_owned()];
    let mut edges = Vec::new();
    while keys.last().unwrap() != start {
        let (parent, ordinal) = &walk.parents[keys.last().unwrap()];
        keys.push(parent.clone());
        edges.push(walk.edges[ordinal].clone());
    }
    keys.reverse();
    edges.reverse();
    json!({"status": "found", "path": keys, "pathEdges": edges})
}
