use chrono::NaiveDate;
use sqlx::{Row, SqlitePool};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use crate::{User, UserId, Result, UserError, UserDatabases};

/// Zentrale Verwaltung aller Benutzer
pub struct UserDatabase {
    pool: SqlitePool,
    db_path: PathBuf,
    base_data_path: PathBuf,
}

impl UserDatabase {
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        let needs_init = !db_path.exists();

        let base_data_path = db_path.parent()
            .ok_or_else(|| UserError::InvalidData("Invalid database path".to_string()))?
            .to_path_buf();

        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let mut db = Self { pool, db_path, base_data_path };

        if needs_init {
            db.initialize().await?;
        }

        Ok(db)
    }

    async fn initialize(&mut self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                first_name TEXT NOT NULL,
                last_name TEXT NOT NULL,
                middle_name TEXT,
                date_of_birth TEXT NOT NULL,
                public_db_path TEXT NOT NULL,
                private_db_path TEXT NOT NULL,
                created_at TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_users_name
             ON users(last_name, first_name)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub fn base_data_path(&self) -> &Path {
        &self.base_data_path
    }

    pub async fn create_user(
        &self,
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        date_of_birth: NaiveDate,
    ) -> Result<(User, UserDatabases)> {
        let user = User::new(first_name, last_name, middle_name, date_of_birth)?;
        user.validate()?;

        let user_dbs = UserDatabases::open(&self.base_data_path, user.id()).await?;
        user_dbs.save_user_info(&user).await?;

        sqlx::query(
            "INSERT INTO users (
                id, first_name, last_name, middle_name, date_of_birth,
                public_db_path, private_db_path, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(&user.id)
        .bind(&user.first_name)
        .bind(&user.last_name)
        .bind(&user.middle_name)
        .bind(user.date_of_birth.to_string())
        .bind(user_dbs.public_db_path().to_string_lossy().to_string())
        .bind(user_dbs.private_db_path().to_string_lossy().to_string())
        .bind(user.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        let _key_meta = user_dbs.add_key_pair().await?;

        Ok((user, user_dbs))
    }

    pub async fn open_user_databases(&self, user_id: &UserId) -> Result<UserDatabases> {
        UserDatabases::open(&self.base_data_path, user_id).await.map_err(Into::into)
    }

    pub async fn get_user(&self, id: &UserId) -> Result<User> {
        let row = sqlx::query(
            "SELECT id, first_name, last_name, middle_name, date_of_birth, created_at
             FROM users WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let row = row.ok_or_else(|| UserError::UserNotFound(id.inner().to_string()))?;

        let date_str: String = row.try_get(4)?;
        let date_of_birth = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| UserError::InvalidData(e.to_string()))?;

        let created_str: String = row.try_get(5)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)
            .map_err(|e| UserError::InvalidData(e.to_string()))?
            .with_timezone(&chrono::Utc);

        Ok(User {
            id: row.try_get(0)?,
            first_name: row.try_get(1)?,
            last_name: row.try_get(2)?,
            middle_name: row.try_get(3)?,
            date_of_birth,
            created_at,
        })
    }

    pub async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, first_name, last_name, middle_name, date_of_birth, created_at
             FROM users ORDER BY last_name, first_name",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            let date_str: String = row.try_get(4)?;
            let date_of_birth = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                .map_err(|e| UserError::InvalidData(e.to_string()))?;

            let created_str: String = row.try_get(5)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_str)
                .map_err(|e| UserError::InvalidData(e.to_string()))?
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

    pub async fn delete_user(&self, id: &UserId) -> Result<()> {
        let result = sqlx::query("DELETE FROM users WHERE id = ?1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(UserError::UserNotFound(id.inner().to_string()));
        }
        Ok(())
    }

    pub async fn update_user(&self, id: &UserId, first_name: String, last_name: String,
        middle_name: Option<String>, date_of_birth: NaiveDate) -> Result<User> {
        let result = sqlx::query(
            "UPDATE users SET first_name = ?2, last_name = ?3, middle_name = ?4, date_of_birth = ?5
             WHERE id = ?1",
        )
        .bind(id)
        .bind(first_name)
        .bind(last_name)
        .bind(middle_name)
        .bind(date_of_birth.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(UserError::UserNotFound(id.inner().to_string()));
        }

        self.get_user(id).await
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub async fn count_users(&self) -> Result<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as usize)
    }
}
