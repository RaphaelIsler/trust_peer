use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Object(String),
}

impl Value {
    pub fn value_type(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Boolean(_) => "boolean",
            Value::Object(_) => "object",
        }
    }

    pub fn to_db_parts(&self) -> (String, String) {
        let value = match self {
            Value::String(value) => value.clone(),
            Value::Number(value) => value.to_string(),
            Value::Boolean(value) => value.to_string(),
            Value::Object(value) => value.clone(),
        };
        (value, self.value_type().to_string())
    }

    pub fn from_db_parts(value: String, value_type: String) -> Result<Self> {
        match value_type.as_str() {
            "string" => Ok(Value::String(value)),
            "number" => Ok(Value::Number(value.parse::<f64>().map_err(|err| {
                anyhow!("Invalid number value '{}': {}", value, err)
            })?)),
            "boolean" => Ok(Value::Boolean(value.parse::<bool>().map_err(|err| {
                anyhow!("Invalid boolean value '{}': {}", value, err)
            })?)),
            "object" => Ok(Value::Object(value)),
            other => Err(anyhow!("Unknown value type '{}'.", other)),
        }
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Value::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Value::String(value.to_string())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Number(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Number(value as f64)
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Number(value as f64)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Number(value as f64)
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Value::Number(value as f64)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Value::Boolean(value)
    }
}

impl TryFrom<&Value> for String {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self> {
        match value {
            Value::String(value) => Ok(value.clone()),
            Value::Object(value) => Ok(value.clone()),
            other => Err(anyhow!("Expected string, got {:?}", other)),
        }
    }
}

impl TryFrom<&Value> for bool {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self> {
        match value {
            Value::Boolean(value) => Ok(*value),
            other => Err(anyhow!("Expected boolean, got {:?}", other)),
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self> {
        match value {
            Value::Number(value) => Ok(*value),
            other => Err(anyhow!("Expected number, got {:?}", other)),
        }
    }
}

impl TryFrom<&Value> for i64 {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self> {
        match value {
            Value::Number(value) if value.fract() == 0.0 => Ok(*value as i64),
            Value::Number(value) => Err(anyhow!("Expected integer number, got {}", value)),
            other => Err(anyhow!("Expected number, got {:?}", other)),
        }
    }
}

impl TryFrom<&Value> for u64 {
    type Error = anyhow::Error;

    fn try_from(value: &Value) -> Result<Self> {
        match value {
            Value::Number(value) if value.fract() == 0.0 && *value >= 0.0 => Ok(*value as u64),
            Value::Number(value) => Err(anyhow!("Expected unsigned integer, got {}", value)),
            other => Err(anyhow!("Expected number, got {:?}", other)),
        }
    }
}

/// Simple key-value storage entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: Value,
}

impl KeyValue {
    pub fn new_typed<T: Into<Value>>(key: String, value: T) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }

    pub fn set_value<T: Into<Value>>(&mut self, value: T) {
        self.value = value.into();
    }

    pub fn get_value<T: for<'a> TryFrom<&'a Value, Error = anyhow::Error>>(&self) -> Result<T> {
        T::try_from(&self.value)
    }
}

impl db::DbEntity for KeyValue {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.key
    }

    fn table_name() -> &'static str {
        "user_key_values"
    }

    fn schema_version() -> u32 {
        1
    }

    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_key_values (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                value_type TEXT NOT NULL
            );",
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn update_table(_conn: &SqlitePool, _from_version: u32, _to_version: u32) -> Result<()> {
        Ok(())
    }

    async fn write(&self, conn: &SqlitePool) -> Result<()> {
        let key = self.key.clone();
        let (value, value_type) = self.value.to_db_parts();

        sqlx::query(
            "INSERT OR REPLACE INTO user_key_values (key, value, value_type)
             VALUES (?1, ?2, ?3)",
        )
        .bind(key)
        .bind(value)
        .bind(value_type)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let row = sqlx::query(
            "SELECT key, value, value_type FROM user_key_values WHERE key = ?1",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            let value: String = row.try_get(1)?;
            let value_type: String = row.try_get(2)?;
            Ok(Some(KeyValue {
                key: row.try_get(0)?,
                value: Value::from_db_parts(value, value_type)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM user_key_values WHERE key = ?1")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query("SELECT key, value, value_type FROM user_key_values ORDER BY key")
            .fetch_all(conn)
            .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let value: String = row.try_get(1)?;
            let value_type: String = row.try_get(2)?;
            items.push(KeyValue {
                key: row.try_get(0)?,
                value: Value::from_db_parts(value, value_type)?,
            });
        }

        Ok(items)
    }
}
