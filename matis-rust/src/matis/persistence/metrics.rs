use super::navigation_queries::metric_value;
use super::{
    database::run,
    graph_inserts,
    models::{GraphNodeMetricRow, graph_node_metrics},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::NodeMetricsPort};
use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use std::collections::HashMap;
pub(crate) struct Store(pub Session);

fn metric_record(id: &str, key: &str, n: &Value) -> GraphNodeMetricRow {
    let p = &n["properties"];
    GraphNodeMetricRow {
        repository_id: id.into(),
        node_key: key.into(),
        line_count: p["lineCount"].as_i64(),
        parameter_count: p["parameterCount"].as_i64(),
        base_complexity: p["baseCyclomaticComplexity"].as_i64(),
        nested_complexity: p["nestedCyclomaticComplexity"].as_i64(),
        loop_complexity: p["loopNestedComplexity"].as_i64(),
        total_complexity: p["cyclomaticComplexity"].as_i64(),
        max_decision_depth: p["maxDecisionDepth"].as_i64(),
    }
}

impl NodeMetricsPort for Store {
    fn all(&self, id: &str) -> Result<HashMap<String, Value>> {
        self.0.with(|c| {
            run(async {
                Ok(graph_node_metrics::Entity::find()
                    .filter(graph_node_metrics::Column::RepositoryId.eq(id))
                    .all(c)
                    .await?
                    .into_iter()
                    .map(|m| (m.node_key.clone(), metric_value(m)))
                    .collect())
            })
        })
    }
    fn get(&self, id: &str, key: &str) -> Result<Value> {
        self.0.with(|c| {
            run(async {
                Ok(graph_node_metrics::Entity::find_by_id((id.to_owned(), key.to_owned()))
                    .one(c)
                    .await?
                    .map(metric_value)
                    .unwrap_or(Value::Null))
            })
        })
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        let rows = collection(artifact, "nodes")
            .iter()
            .filter(|n| METRIC_NAMES.iter().any(|name| n["properties"].get(name).is_some()))
            .map(|n| Ok(metric_record(id, &required(n, "stableKey")?, n)));
        self.0.with(|c| run(graph_inserts::insert::<graph_node_metrics::Entity>(c, rows)))
    }
}
