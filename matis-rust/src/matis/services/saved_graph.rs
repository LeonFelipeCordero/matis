use super::{
    diagnostics::DiagnosticService, graph_revisions::GraphRevisionService,
    mutation_candidates::MutationCandidateService, node_metrics::NodeMetricsService, nodes::NodeService,
    relationships::RelationshipService,
};
use crate::domain::{
    graph_artifact::METRIC_NAMES,
    repositories::{GraphDto, GraphStatsDto},
};
use anyhow::Result;
use serde_json::{Value, json};
use std::collections::HashMap;

// Composes capabilities; each capability alone owns its table ports.
pub struct SavedGraphWorkflow {
    pub revisions: GraphRevisionService,
    pub nodes: NodeService,
    pub relationships: RelationshipService,
    pub metrics: NodeMetricsService,
    pub diagnostics: DiagnosticService,
    pub candidates: MutationCandidateService,
}
impl SavedGraphWorkflow {
    pub(crate) fn summary(&self, id: &str) -> Result<Option<GraphDto>> {
        self.revisions.get(id)?.map(|g| self.with_stats(g)).transpose()
    }
    fn with_stats(&self, mut graph: GraphDto) -> Result<GraphDto> {
        let id = &graph.repository_id;
        let levels: HashMap<_, _> = self.nodes.levels(id)?.into_iter().collect();
        let level = |name: &str| *levels.get(name).unwrap_or(&0);
        graph.stats = GraphStatsDto {
            files: level("file"),
            symbols: self.nodes.symbol_count(id)?,
            modules: level("module"),
            types: level("type"),
            callables: level("callable"),
            nodes: levels.values().sum(),
            edges: self.relationships.counts(id)?.into_iter().map(|(_, count)| count).sum(),
            external_nodes: level("external"),
            parse_errors: self.diagnostics.errors(id)?,
        };
        Ok(graph)
    }
    pub(crate) fn report_facts(&self, id: &str) -> Result<Option<Value>> {
        let Some(graph) = self.revisions.get(id)? else { return Ok(None) };
        let metrics = self.metrics.all(id)?;
        let nodes = self
            .nodes
            .all(id)?
            .into_iter()
            .map(|mut node| {
                let metric = metrics.get(node["stableKey"].as_str().unwrap()).unwrap_or(&Value::Null);
                for field in ["language", "startColumn", "endLine", "endColumn", "signature", "visibility"] {
                    node.as_object_mut().unwrap().remove(field);
                }
                for name in METRIC_NAMES {
                    node[name] = metric[name].clone();
                }
                node
            })
            .collect::<Vec<_>>();
        let edges = self.relationships.all(id)?;
        let imports: Vec<_> = edges.iter().filter(|e| e["type"] == "IMPORTS").cloned().collect();
        Ok(Some(json!({"graphRevision":graph.graph_revision,"nodes":nodes,"edges":edges,"imports":imports,
            "diagnostics":self.diagnostics.all(id)?,"mutationCandidates":self.candidates.all(id)?})))
    }
    pub(crate) fn mutation_candidates(&self, id: &str) -> Result<Option<Vec<Value>>> {
        self.revisions.get(id)?.map(|_| self.candidates.all(id)).transpose()
    }
    pub(crate) fn save(&self, id: &str, artifact: &Value) -> Result<GraphDto> {
        let graph = self.revisions.save(id, artifact)?;
        self.relationships.clear(id)?;
        // Node deletion cascades to its dependent rows, including old candidate file references.
        self.nodes.replace(id, artifact)?;
        self.metrics.insert(id, artifact)?;
        self.candidates.insert(id, artifact)?;
        self.relationships.insert(id, artifact)?;
        self.diagnostics.replace(id, artifact)?;
        self.with_stats(graph)
    }
}
