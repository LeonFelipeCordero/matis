use super::{
    database::run,
    graph_inserts,
    models::{GraphNodeDecoratorRow, graph_node_decorators},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::DecoratorPort};
use anyhow::{Result, anyhow};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;

pub(crate) struct Store(pub Session);

impl DecoratorPort for Store {
    fn get(&self, id: &str, key: &str, limit: usize) -> Result<Vec<String>> {
        self.0.with(|c| {
            run(async {
                Ok(graph_node_decorators::Entity::find()
                    .filter(graph_node_decorators::Column::RepositoryId.eq(id))
                    .filter(graph_node_decorators::Column::NodeKey.eq(key))
                    .select_only()
                    .column(graph_node_decorators::Column::Decorator)
                    .order_by_asc(graph_node_decorators::Column::Ordinal)
                    .limit(limit as u64)
                    .into_tuple::<String>()
                    .all(c)
                    .await?)
            })
        })
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        let rows = collection(artifact, "nodes").iter().filter(|n| is_symbol(n)).flat_map(|n| {
            collection(&n["properties"], "decorators").iter().enumerate().map(move |(ordinal, d)| {
                Ok(GraphNodeDecoratorRow {
                    repository_id: id.into(),
                    node_key: required(n, "stableKey")?,
                    ordinal: ordinal as i64,
                    decorator: d.as_str().ok_or_else(|| anyhow!("Graph decorators must contain strings"))?.into(),
                })
            })
        });
        self.0.with(|c| run(graph_inserts::insert::<graph_node_decorators::Entity>(c, rows)))
    }
}
