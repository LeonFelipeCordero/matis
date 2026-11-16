use super::{
    database::run,
    graph_inserts,
    models::{GraphMutationCandidateRow, graph_mutation_candidates},
    session::Session,
};
use crate::domain::{graph_artifact::*, storage::MutationCandidatePort};
use anyhow::{Result, anyhow};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::Value;
use std::collections::HashMap;

pub(crate) struct Store(pub Session);

impl MutationCandidatePort for Store {
    fn all(&self, id: &str) -> Result<Vec<Value>> {
        self.0.with(|c| {
            run(async {
                Ok(graph_mutation_candidates::Entity::find()
                    .filter(graph_mutation_candidates::Column::RepositoryId.eq(id))
                    .order_by_asc(graph_mutation_candidates::Column::Ordinal)
                    .select_only()
                    .column(graph_mutation_candidates::Column::Payload)
                    .into_tuple::<String>()
                    .all(c)
                    .await?
                    .into_iter()
                    .map(|s| serde_json::from_str(&s))
                    .collect::<std::result::Result<Vec<_>, _>>()?)
            })
        })
    }
    fn insert(&self, id: &str, artifact: &Value) -> Result<()> {
        let files: HashMap<_, _> = collection(artifact, "nodes")
            .iter()
            .filter(|n| n["level"] == "file")
            .map(|n| Ok((required(n, "path")?, required(n, "stableKey")?)))
            .collect::<Result<_>>()?;
        let rows = collection(artifact, "mutationCandidates").iter().enumerate().map(|(ordinal, c)| {
            let path = required(c, "path")?;
            Ok(GraphMutationCandidateRow {
                repository_id: id.into(),
                candidate_id: required(c, "id")?,
                file_key: files.get(&path).ok_or_else(|| anyhow!("Candidate file not found: {path}"))?.clone(),
                ordinal: ordinal as i64,
                path,
                language: required(c, "language")?,
                source_hash: required(c, "sourceHash")?,
                kind: required(c, "kind")?,
                operator: text(c, "operator"),
                payload: serde_json::to_string(c)?,
            })
        });
        self.0.with(|c| run(graph_inserts::insert::<graph_mutation_candidates::Entity>(c, rows)))
    }
}
