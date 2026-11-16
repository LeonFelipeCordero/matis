use super::{
    database::run,
    graph_inserts,
    models::{GraphSymbolRow, graph_symbols},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::SymbolPort};
use anyhow::Result;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::{Value, json};
use std::collections::HashMap;
pub(crate) struct Store(pub Session);

fn symbol_record(id: &str, key: &str, n: &Value) -> GraphSymbolRow {
    GraphSymbolRow {
        repository_id: id.into(),
        node_key: key.into(),
        start_line: n["startLine"].as_i64(),
        start_column: n["startColumn"].as_i64(),
        end_line: n["endLine"].as_i64(),
        end_column: n["endColumn"].as_i64(),
        signature: text(n, "signature"),
        visibility: text(&n["properties"], "visibility"),
    }
}

async fn read_many(c: &impl sea_orm::ConnectionTrait, id: &str, keys: &[String]) -> Result<HashMap<String, Value>> {
    let mut result = HashMap::new();
    for batch in keys.chunks(900) {
        result.extend(
            graph_symbols::Entity::find()
                .filter(graph_symbols::Column::RepositoryId.eq(id))
                .filter(graph_symbols::Column::NodeKey.is_in(batch.iter().cloned()))
                .all(c)
                .await?
                .into_iter()
                .map(|s| (s.node_key.clone(), value(s))),
        );
    }
    Ok(result)
}

fn value(s: GraphSymbolRow) -> Value {
    json!({"startLine":s.start_line,"startColumn":s.start_column,"endLine":s.end_line,
        "endColumn":s.end_column,"signature":s.signature,"visibility":s.visibility})
}
impl SymbolPort for Store {
    fn get_many(&self, id: &str, keys: &[String]) -> Result<HashMap<String, Value>> {
        self.0.with(|c| run(read_many(c, id, keys)))
    }
    fn count(&self, id: &str) -> Result<i64> {
        self.0.with(|c| {
            run(async {
                Ok(graph_symbols::Entity::find().filter(graph_symbols::Column::RepositoryId.eq(id)).count(c).await?
                    as i64)
            })
        })
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        let rows = collection(artifact, "nodes")
            .iter()
            .filter(|n| is_symbol(n))
            .map(|n| Ok(symbol_record(id, &required(n, "stableKey")?, n)));
        self.0.with(|c| run(graph_inserts::insert::<graph_symbols::Entity>(c, rows)))
    }
}
