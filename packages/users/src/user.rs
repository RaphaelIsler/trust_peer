use anyhow::Result;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

pub type UserId = helper::UId<User>;

/// Repräsentiert einen Benutzer im System
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Eindeutige Benutzer-ID
    pub id: UserId,

    /// Vorname (erforderlich)
    pub first_name: String,

    /// Nachname (erforderlich)
    pub last_name: String,

    /// Optionaler zweiter Vorname
    pub middle_name: Option<String>,

    /// Geburtsdatum
    pub date_of_birth: NaiveDate,

    /// Erstellungszeitpunkt
    pub created_at: chrono::DateTime<chrono::Utc>,
}


impl User {
    /// Erstellt einen neuen Benutzer
    pub fn new(
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        date_of_birth: NaiveDate,
    ) -> crate::Result<Self> {
        let id = UserId::new();

        Ok(Self {
            id,
            first_name,
            last_name,
            middle_name,
            date_of_birth,
            created_at: chrono::Utc::now(),
        })
    }

    /// Gibt die ID zurück
    pub fn id(&self) -> &UserId {
        &self.id
    }

    /// Gibt den vollständigen Namen zurück
    pub fn full_name(&self) -> String {
        if let Some(ref middle) = self.middle_name {
            format!("{} {} {}", self.first_name, middle, self.last_name)
        } else {
            format!("{} {}", self.first_name, self.last_name)
        }
    }

    /// Validiert die Benutzerdaten
    pub fn validate(&self) -> crate::Result<()> {
        if self.first_name.trim().is_empty() {
            return Err(crate::UserError::InvalidData(
                "Vorname darf nicht leer sein".to_string(),
            ));
        }

        if self.last_name.trim().is_empty() {
            return Err(crate::UserError::InvalidData(
                "Nachname darf nicht leer sein".to_string(),
            ));
        }

        // Prüfe ob Geburtsdatum in der Vergangenheit liegt
        let today = chrono::Utc::now().date_naive();
        if self.date_of_birth >= today {
            return Err(crate::UserError::InvalidData(
                "Geburtsdatum muss in der Vergangenheit liegen".to_string(),
            ));
        }

        Ok(())
    }
}

impl db::DbEntity for User {
    type Id = UserId;

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
                middle_name TEXT,
                date_of_birth TEXT NOT NULL,
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

    async fn update_table(_conn: &SqlitePool, _from_version: u32, _to_version: u32) -> anyhow::Result<()> {
        // Keine Migrationen erforderlich (noch)
        Ok(())
    }

    async fn write(&self, conn: &SqlitePool) -> anyhow::Result<()> {
        let id = self.id.clone();
        let first_name = self.first_name.clone();
        let last_name = self.last_name.clone();
        let middle_name = self.middle_name.clone();
        let date_of_birth = self.date_of_birth.to_string();
        let created_at = self.created_at.to_rfc3339();

        sqlx::query(
            "INSERT OR REPLACE INTO users (
                id, first_name, last_name, middle_name, date_of_birth, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(id)
        .bind(first_name)
        .bind(last_name)
        .bind(middle_name)
        .bind(date_of_birth)
        .bind(created_at)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let row = sqlx::query(
            "SELECT id, first_name, last_name, middle_name, date_of_birth, created_at
             FROM users WHERE id = ?1",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            let date_str: String = row.try_get(4)?;
            let date_of_birth = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;

            let created_str: String = row.try_get(5)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)?
                .with_timezone(&chrono::Utc);

            Ok(Some(User {
                id: row.try_get(0)?,
                first_name: row.try_get(1)?,
                last_name: row.try_get(2)?,
                middle_name: row.try_get(3)?,
                date_of_birth,
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
            "SELECT id, first_name, last_name, middle_name, date_of_birth, created_at
             FROM users ORDER BY last_name, first_name",
        )
        .fetch_all(conn)
        .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let date_str: String = row.try_get(4)?;
            let date_of_birth = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;

            let created_str: String = row.try_get(5)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)?
                .with_timezone(&chrono::Utc);

            users.push(User {
                id: row.try_get(0)?,
                first_name: row.try_get(1)?,
                last_name: row.try_get(2)?,
                middle_name: row.try_get(3)?,
                date_of_birth,
                created_at,
            });
        }

        Ok(users)
    }
}
