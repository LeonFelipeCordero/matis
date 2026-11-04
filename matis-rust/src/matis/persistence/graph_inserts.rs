use anyhow::Result;
use sea_orm::{ConnectionTrait, DbErr, EntityTrait, IntoActiveModel};

// The widest graph row has 11 columns: 90 rows use 990 of SQLite's conservative 999 binds.
const BATCH_ROWS: usize = 90;

pub(super) async fn insert<E: EntityTrait>(
    connection: &impl ConnectionTrait,
    mut rows: impl Iterator<Item = Result<E::Model>>,
) -> Result<()>
where
    E::Model: IntoActiveModel<E::ActiveModel>,
{
    loop {
        let batch = rows
            .by_ref()
            .take(BATCH_ROWS)
            .map(|row| row.map(IntoActiveModel::into_active_model))
            .collect::<Result<Vec<_>>>()?;
        if batch.is_empty() {
            return Ok(());
        }
        let expected = batch.len() as u64;
        let affected = E::insert_many(batch).exec_without_returning(connection).await?;
        if affected != expected {
            return Err(DbErr::RecordNotInserted.into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::models::*;
    use sea_orm::Iterable;

    fn assert_bind_budget<E: EntityTrait>() {
        assert!(BATCH_ROWS * E::Column::iter().count() <= 999);
    }

    #[test]
    fn graph_batches_stay_within_bind_budget() {
        assert_bind_budget::<graph_nodes::Entity>();
        assert_bind_budget::<graph_files::Entity>();
        assert_bind_budget::<graph_symbols::Entity>();
        assert_bind_budget::<graph_node_metrics::Entity>();
        assert_bind_budget::<graph_node_decorators::Entity>();
        assert_bind_budget::<graph_edges::Entity>();
        assert_bind_budget::<graph_diagnostics::Entity>();
        assert_bind_budget::<graph_mutation_candidates::Entity>();
    }
}
