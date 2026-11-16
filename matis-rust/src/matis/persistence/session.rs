use super::{
    database::{create_engine, run, storage_error},
    navigation_database::{ReadTransaction, readonly_session},
};
use crate::domain::storage::{Transaction, UnitOfWork};
use anyhow::{Result, ensure};
use sea_orm::{DatabaseConnection, DatabaseTransaction, TransactionTrait};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

struct State {
    database: Option<DatabaseConnection>,
    transaction: Option<ReadTransaction>,
}
#[derive(Clone)]
pub(crate) struct Session(Arc<Mutex<State>>);
impl Session {
    pub fn writable(path: Option<&Path>) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(State { database: Some(create_engine(path)?), transaction: None }))))
    }
    pub fn readonly(path: Option<&Path>) -> Result<Self> {
        Ok(Self(Arc::new(Mutex::new(State { database: None, transaction: Some(readonly_session(path)?) }))))
    }
    pub fn with<T>(&self, operation: impl FnOnce(&DatabaseTransaction) -> Result<T>) -> Result<T> {
        let state = self.0.lock().expect("storage session");
        operation(state.transaction.as_ref().ok_or_else(|| anyhow::anyhow!("Storage requires a transaction"))?)
    }
}
struct Scope(Session);
impl UnitOfWork for Session {
    fn begin(&self) -> Result<Box<dyn Transaction>> {
        let mut state = self.0.lock().expect("storage session");
        ensure!(state.transaction.is_none(), "Nested storage transaction");
        let db = state.database.as_ref().ok_or_else(|| anyhow::anyhow!("Read-only storage session"))?;
        state.transaction = Some(ReadTransaction::new(run(db.begin()).map_err(storage_error)?));
        Ok(Box::new(Scope(self.clone())))
    }
}
impl Transaction for Scope {
    fn commit(self: Box<Self>) -> Result<()> {
        let tx = self.0.0.lock().expect("storage session").transaction.take().expect("active transaction");
        run(tx.take().commit()).map_err(storage_error)
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        let tx = self.0.0.lock().expect("storage session").transaction.take();
        drop(tx);
    }
}
