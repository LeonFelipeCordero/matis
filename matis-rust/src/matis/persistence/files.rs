use super::{
    database::run,
    graph_inserts,
    models::{GraphFileRow, graph_files},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::FilePort};
use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use std::collections::HashMap;

pub(crate) struct Store(pub Session);

async fn read_many(c: &impl sea_orm::ConnectionTrait, id: &str, keys: &[String]) -> Result<HashMap<String, Value>> {
    let mut result = HashMap::new();
    for batch in keys.chunks(900) {
        result.extend(
            graph_files::Entity::find()
                .filter(graph_files::Column::RepositoryId.eq(id))
                .filter(graph_files::Column::NodeKey.is_in(batch.iter().cloned()))
                .all(c)
                .await?
                .into_iter()
                .map(|f| (f.node_key, json!({"bytes":f.byte_count,"sha256":f.sha256}))),
        );
    }
    Ok(result)
}

impl FilePort for Store {
    fn get_many(&self, id: &str, keys: &[String]) -> Result<HashMap<String, Value>> {
        self.0.with(|c| run(read_many(c, id, keys)))
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        let rows = collection(artifact, "nodes").iter().filter(|n| n["level"] == "file").map(|n| {
            Ok(GraphFileRow {
                repository_id: id.into(),
                node_key: required(n, "stableKey")?,
                byte_count: n["properties"]["bytes"].as_i64(),
                sha256: text(&n["properties"], "sha256"),
            })
        });
        self.0.with(|c| run(graph_inserts::insert::<graph_files::Entity>(c, rows)))
    }
}
