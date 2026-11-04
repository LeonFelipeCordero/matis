use super::{
    navigation_source::source_excerpt,
    navigation_walk::{route, traverse},
};
use crate::domain::navigation::{NavigationReader, PathRequest, SourceOptions, TraversalRequest};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub struct NavigationService {
    reader: Box<dyn NavigationReader>,
}
impl NavigationService {
    /// Own an existing snapshot reader without querying it or accessing source files.
    pub fn new(reader: impl NavigationReader + 'static) -> Self {
        Self { reader: Box::new(reader) }
    }
    /// Merge snapshot metadata, `filters`, and object `payload`, adding up to 30 diagnostics for `nodes` paths.
    /// Propagate diagnostic read errors; set a truncation flag after lookahead.
    ///
    /// # Panics
    /// Panic if `payload` is not a JSON object.
    pub fn response(&mut self, payload: Value, filters: Value, nodes: &[Value]) -> Result<Value> {
        let paths: Vec<_> = nodes
            .iter()
            .filter_map(|n| n["path"].as_str().filter(|p| !p.is_empty()).map(str::to_owned))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut diagnostics = if paths.is_empty() { vec![] } else { self.reader.diagnostics(&paths, 31)? };
        let truncated = diagnostics.len() > 30;
        diagnostics.truncate(30);
        let mut value = self.reader.snapshot().metadata();
        value["filters"] = filters;
        value.as_object_mut().unwrap().extend(payload.as_object().unwrap().clone());
        value["diagnostics"] = json!(diagnostics);
        value["diagnosticsTruncated"] = json!(truncated);
        Ok(value)
    }
    /// Return graph counts and bounded roots with snapshot metadata and diagnostics.
    /// Propagate reader errors without writing storage.
    pub fn overview(&mut self) -> Result<Value> {
        let payload = self.reader.overview()?;
        let nodes = payload["roots"].as_array().cloned().unwrap_or_default();
        self.response(payload, json!({"rootLimit": 30}), &nodes)
    }
    /// Return up to `limit` ranked matches for `query` with metadata and a lookahead truncation flag.
    /// Propagate node/diagnostic read errors; callers must supply a limit that permits `limit + 1`.
    pub fn find(&mut self, query: &str, limit: usize) -> Result<Value> {
        let mut nodes = self.reader.find(query, limit + 1)?;
        let truncated = nodes.len() > limit;
        nodes.truncate(limit);
        self.response(json!({"nodes": nodes, "truncated": truncated}), json!({"query": query, "limit": limit}), &nodes)
    }
    /// Return nearest-first parents and whether a cycle or the 100-parent cap stopped traversal.
    /// Propagate missing-parent/read errors; require a string `stableKey` in `node` or panic.
    pub fn ancestors(&mut self, node: &Value) -> Result<(Vec<Value>, bool)> {
        let mut nodes = vec![];
        let mut seen = BTreeSet::from([node["stableKey"].as_str().unwrap().to_owned()]);
        let mut parent = node["parentStableKey"].as_str().map(str::to_owned);
        while let Some(key) = parent.clone().filter(|k| !seen.contains(k) && nodes.len() < 100) {
            seen.insert(key.clone());
            let ancestor = self.reader.node(&key)?;
            parent = ancestor["parentStableKey"].as_str().map(str::to_owned);
            nodes.push(ancestor);
        }
        Ok((nodes, parent.is_some()))
    }
    /// Return stable-key details, bounded ancestors, metadata, and diagnostics.
    /// Propagate missing-node, missing-parent, or other reader errors.
    pub fn show(&mut self, key: &str) -> Result<Value> {
        let node = self.reader.node(key)?;
        let (ancestors, truncated) = self.ancestors(&node)?;
        let mut nodes = vec![node.clone()];
        nodes.extend(ancestors.clone());
        self.response(
            json!({"node": node, "ancestors": ancestors, "truncated": truncated}),
            json!({"key": key, "ancestorLimit": 100}),
            &nodes,
        )
    }
    /// Return a child page for `key` with truncation, continuation offset, and diagnostics.
    /// Require a nonnegative SQLite-range integer `offset` and a `limit` allowing lookahead.
    /// Return errors for absent parents, invalid offsets, or reader failures.
    pub fn children(&mut self, key: &str, limit: usize, offset: serde_json::Number) -> Result<Value> {
        let node = self.reader.node(key)?;
        let database_offset = offset
            .as_i64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| crate::domain::navigation::error("Python int too large to convert to SQLite INTEGER"))?;
        let mut rows = self.reader.children(key, limit + 1, database_offset)?;
        let truncated = rows.len() > limit;
        rows.truncate(limit);
        let mut nodes = vec![node];
        nodes.extend(rows.clone());
        self.response(json!({"nodes": rows, "truncated": truncated, "nextOffset": if truncated { Some(database_offset + limit) } else { None }}),
            json!({"key": key, "limit": limit, "offset": offset}), &nodes)
    }
    /// Return bounded breadth-first context, cutoff reasons, metadata, and diagnostics.
    /// Use `request` bounds and filters as supplied; propagate missing-node/read errors.
    pub fn context(&mut self, request: &TraversalRequest) -> Result<Value> {
        let walk = traverse(self.reader.as_mut(), request)?;
        let filters = json!({"key": request.key, "direction": request.direction, "relations": request.relations,
            "depth": request.depth, "maxNodes": request.max_nodes, "maxEdges": request.max_edges});
        self.response(walk.output(), filters, &walk.nodes.into_values().collect::<Vec<_>>())
    }
    /// Return a shortest discovered route with traversal bounds, metadata, and diagnostics.
    /// Validate both endpoint keys through the reader; propagate reader errors.
    /// Unreached targets are `unknown` after cutoffs, otherwise `not_found`.
    pub fn path(&mut self, request: &PathRequest) -> Result<Value> {
        self.reader.node(&request.target)?;
        let options = &request.traversal;
        let walk = traverse(self.reader.as_mut(), options)?;
        let filters = json!({"from": options.key, "to": request.target, "direction": options.direction, "relations": options.relations,
            "maxDepth": options.depth, "maxNodes": options.max_nodes, "maxEdges": options.max_edges});
        let mut payload = walk.output();
        payload
            .as_object_mut()
            .unwrap()
            .extend(route(&walk, &options.key, &request.target).as_object().unwrap().clone());
        self.response(payload, filters, &walk.nodes.into_values().collect::<Vec<_>>())
    }
    /// Read a hash-verified local excerpt for `key`, with metadata and per-source status.
    /// Bound output by the node span, `max_lines`, and byte limits; use one-based
    /// `start_line` or the span start. Propagate missing-key/read errors; never return unverified source.
    pub fn source(&mut self, key: &str, max_lines: usize, start_line: Option<serde_json::Number>) -> Result<Value> {
        let node = self.reader.node(key)?;
        let excerpt =
            source_excerpt(self.reader.as_mut(), &node, &SourceOptions { max_lines, start_line: start_line.clone() })?;
        self.response(
            json!({"source": excerpt}),
            json!({"key": key, "maxLines": max_lines, "startLine": start_line}),
            &[node],
        )
    }
    /// Read verified local excerpts for `keys` in input order with individual statuses.
    /// Apply one-based `start_line` and `max_lines` within each node's span and byte caps.
    /// Resolve all keys before reading files; propagate missing-key or reader errors.
    pub fn sources(
        &mut self,
        keys: &[String],
        max_lines: usize,
        start_line: Option<serde_json::Number>,
    ) -> Result<Value> {
        let nodes = keys.iter().map(|k| self.reader.node(k)).collect::<Result<Vec<_>>>()?;
        let options = SourceOptions { max_lines, start_line: start_line.clone() };
        let mut excerpts = vec![];
        for node in &nodes {
            let mut value = json!({"stableKey": node["stableKey"]});
            value
                .as_object_mut()
                .unwrap()
                .extend(source_excerpt(self.reader.as_mut(), node, &options)?.as_object().unwrap().clone());
            excerpts.push(value);
        }
        self.response(
            json!({"sources": excerpts}),
            json!({"keys": keys, "maxLines": max_lines, "startLine": start_line}),
            &nodes,
        )
    }
}
