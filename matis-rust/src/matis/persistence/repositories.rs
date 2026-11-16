use super::{
    database::{run, storage_error},
    identity::{now, uuid},
    models::repositories,
    session::Session,
};
use crate::domain::{
    repositories::{RepositoryDto, RepositoryError},
    storage::RepositoryPort,
};
use anyhow::Result;
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
    sea_query::{Expr, Func},
};
pub(crate) struct Store(pub Session);
fn dto(r: repositories::Model) -> RepositoryDto {
    RepositoryDto { id: r.id, name: r.name, location: r.location, created_at: r.created_at, imported_at: None }
}
fn name_matches(name: &str) -> sea_orm::sea_query::SimpleExpr {
    Expr::expr(Func::lower(Expr::col(repositories::Column::Name))).eq(Func::lower(Expr::val(name)))
}
impl RepositoryPort for Store {
    fn register(&self, name: &str, location: &str) -> Result<RepositoryDto> {
        let row =
            repositories::Model { id: uuid(false)?, name: name.into(), location: location.into(), created_at: now() };
        self.0
            .with(|c| {
                run(async {
                    repositories::Entity::insert(row.clone().into_active_model()).exec(c).await?;
                    Ok(dto(row))
                })
            })
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("repositories_name_unique") || message.contains("repositories.name") {
                    RepositoryError::new("A repository with this name already exists", 409).into()
                } else {
                    storage_error(error)
                }
            })
    }
    fn list(&self) -> Result<Vec<RepositoryDto>> {
        self.0
            .with(|c| {
                run(async {
                    Ok(repositories::Entity::find()
                        .order_by_asc(repositories::Column::CreatedAt)
                        .order_by_asc(repositories::Column::Id)
                        .all(c)
                        .await?
                        .into_iter()
                        .map(dto)
                        .collect())
                })
            })
            .map_err(storage_error)
    }
    fn exists(&self, id: &str) -> Result<bool> {
        self.0
            .with(|c| run(async { Ok(repositories::Entity::find_by_id(id).one(c).await?.is_some()) }))
            .map_err(storage_error)
    }
    fn resolve(&self, name: &str) -> Result<Option<String>> {
        self.0
            .with(|c| {
                run(async { Ok(repositories::Entity::find().filter(name_matches(name)).one(c).await?.map(|r| r.id)) })
            })
            .map_err(storage_error)
    }
    fn select(&self, name: &str) -> Result<Vec<RepositoryDto>> {
        self.0.with(|c| {
            run(async {
                Ok(repositories::Entity::find()
                    .filter(Condition::any().add(repositories::Column::Id.eq(name)).add(name_matches(name)))
                    .order_by_asc(repositories::Column::Id)
                    .limit(2)
                    .all(c)
                    .await?
                    .into_iter()
                    .map(dto)
                    .collect())
            })
        })
    }
}
