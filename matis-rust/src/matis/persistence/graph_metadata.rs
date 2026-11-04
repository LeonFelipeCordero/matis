use anyhow::{Result, bail, ensure};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::Value;

pub struct GraphMetadata {
    pub schema_version: Option<i64>,
    pub indexed_at: Option<String>,
    pub root_name: Option<String>,
    pub root_path: Option<String>,
}
/// Test Python-style falseness for JSON null, false, zero, and empty containers/strings.
pub fn falsey(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Bool(value) => !value,
        Value::Number(number) => number.as_f64() == Some(0.0),
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) => value.is_empty(),
    }
}
/// Accept an object or any Python-falsey value; otherwise return an error naming `name`.
pub fn optional_mapping(value: &Value, name: &str) -> Result<()> {
    ensure!(value.is_object() || falsey(value), "Graph {name} must be an object or empty");
    Ok(())
}
impl GraphMetadata {
    /// Convert artifact metadata to SQLite-compatible values without writing rows.
    /// Missing fields become `None`; numeric text conversion may query `connection`.
    ///
    /// # Errors
    /// Reject non-mapping/nonempty repository values, non-scalar metadata, or schema
    /// versions that are not integral SQLite-range values; propagate conversion/query errors.
    pub async fn read(connection: &impl ConnectionTrait, artifact: &Value) -> Result<Self> {
        let repository = &artifact["repository"];
        optional_mapping(repository, "repository")?;
        Ok(Self {
            schema_version: integer(&artifact["schemaVersion"])?,
            indexed_at: scalar_text(connection, &repository["indexedAt"]).await?,
            root_name: scalar_text(connection, &repository["name"]).await?,
            root_path: scalar_text(connection, &repository["path"]).await?,
        })
    }
}
async fn scalar_text(connection: &impl ConnectionTrait, value: &Value) -> Result<Option<String>> {
    let result = match value {
        Value::Null => return Ok(None),
        Value::String(value) => value.clone(),
        Value::Bool(value) => i32::from(*value).to_string(),
        Value::Number(number) => numeric_text(connection, number).await?,
        _ => bail!("Graph metadata fields must be SQLite scalar values or null"),
    };
    Ok(Some(result))
}
async fn numeric_text(connection: &impl ConnectionTrait, number: &serde_json::Number) -> Result<String> {
    if let Some(integer) = number.as_i64() {
        return Ok(integer.to_string());
    }
    ensure!(number.to_string().contains(['.', 'e', 'E']), "Graph metadata integer exceeds SQLite range");
    let real = number.as_f64().ok_or_else(|| anyhow::anyhow!("Graph metadata number exceeds SQLite range"))?;
    Ok(connection
        .query_one(Statement::from_sql_and_values(DbBackend::Sqlite, "SELECT CAST(? AS TEXT) AS value", [real.into()]))
        .await?
        .ok_or_else(|| anyhow::anyhow!("Missing SQLite scalar"))?
        .try_get("", "value")?)
}
fn integer(value: &Value) -> Result<Option<i64>> {
    match value {
        Value::Null => Ok(None),
        Value::Bool(value) => Ok(Some(i64::from(*value))),
        Value::Number(number) => parse_integer(&number.to_string()).map(Some),
        Value::String(text) => parse_integer(text).map(Some),
        _ => bail!("Graph schemaVersion must be an integer or null"),
    }
}
fn parse_integer(text: &str) -> Result<i64> {
    if let Ok(integer) = text.trim().parse::<i64>() {
        return Ok(integer);
    }
    let real = text.trim().parse::<f64>()?;
    ensure!(
        real.is_finite() && real.fract() == 0.0 && real >= i64::MIN as f64 && real < -(i64::MIN as f64),
        "Graph schemaVersion must be a SQLite integer"
    );
    Ok(real as i64)
}
