use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

pub type Id = blockchain::blockchain::Id;

/// Represents a user in the system
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID
    pub id: Id,

    /// First name (required)
    pub first_name: String,

    /// Last name (required)
    pub last_name: String,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl User {
    /// Creates a new user
    pub fn new(first_name: String, last_name: String) -> Self {
        let id = Id::new();

        Self {
            id,
            first_name,
            last_name,
            created_at: chrono::Utc::now(),
        }
    }

    /// Returns the ID
    pub fn id(&self) -> &Id {
        &self.id
    }

    /// Returns the full name
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    /// Validates the user data
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.first_name.trim().is_empty() {
            return Err(crate::UserError::InvalidData(
                "First name cannot be empty".to_string(),
            ));
        }

        if self.last_name.trim().is_empty() {
            return Err(crate::UserError::InvalidData(
                "Last name cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(feature = "frontend")]
#[path = "."]
mod m_frontend {
    use super::*;
    use dioxus::prelude::*;

    #[component]
    pub fn Show(user: User, selected: Option<EventHandler<Id>>) -> Element {
        rsx! {
            div {
                class: "user-card",
                onclick: move |_| {
                    if let Some(handler) = &selected {
                        handler.call(user.id.clone());
                    }
                },
                style: "border: 1px solid #ccc; padding: 10px; border-radius: 5px;",
                h3 { "{user.full_name()}" }
                p { "ID: {user.id.inner()}" }
                p { "Created At: {user.created_at}" }
            }
        }
    }
}
#[cfg(feature = "frontend")]
pub use m_frontend::*;

#[cfg(feature = "backend")]
impl db::DbEntity for User {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn table_name() -> &'static str {
        "users"
    }

    fn schema_version() -> u32 {
        1
    }

    async fn create_table(conn: &SqlitePool) -> anyhow::Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                first_name TEXT NOT NULL,
                last_name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .execute(conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_name ON users(last_name, first_name);")
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn update_table(
        _conn: &SqlitePool,
        _from_version: u32,
        _to_version: u32,
    ) -> anyhow::Result<()> {
        // Keine Migrationen erforderlich (noch)
        Ok(())
    }

    async fn write(&self, conn: &SqlitePool) -> anyhow::Result<()> {
        let id = self.id.clone();
        let first_name = self.first_name.clone();
        let last_name = self.last_name.clone();
        let created_at = self.created_at.to_rfc3339();

        sqlx::query(
            "INSERT OR REPLACE INTO users (
                id, first_name, last_name, created_at
            ) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(id)
        .bind(first_name)
        .bind(last_name)
        .bind(created_at)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let row = sqlx::query(
            "SELECT id, first_name, last_name, created_at
             FROM users WHERE id = ?1",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            let created_str: String = row.try_get(3)?;
            let created_at =
                chrono::DateTime::parse_from_rfc3339(&created_str)?.with_timezone(&chrono::Utc);

            Ok(Some(User {
                id: row.try_get(0)?,
                first_name: row.try_get(1)?,
                last_name: row.try_get(2)?,
                created_at,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> anyhow::Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM users WHERE id = ?1")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> anyhow::Result<Vec<Self>> {
        let rows = sqlx::query(
            "SELECT id, first_name, last_name, created_at
             FROM users ORDER BY last_name, first_name",
        )
        .fetch_all(conn)
        .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let created_str: String = row.try_get(3)?;
            let created_at =
                chrono::DateTime::parse_from_rfc3339(&created_str)?.with_timezone(&chrono::Utc);

            users.push(User {
                id: row.try_get(0)?,
                first_name: row.try_get(1)?,
                last_name: row.try_get(2)?,
                created_at,
            });
        }

        Ok(users)
    }
}
