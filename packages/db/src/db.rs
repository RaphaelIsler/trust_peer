use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::{Path, PathBuf};
use crate::DbEntity;

/// Zentrale Datenbank-Struktur
pub struct DB {
    connection: SqlitePool,
    db_path: PathBuf,
}

impl DB {
    /// Opens a database at the specified path
    /// Creates the file if it doesn't exist
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();

        // Create the parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let connection = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Initialisiere die Migrations-Tabelle
        Self::init_migrations_table(&connection).await?;

        Ok(Self {
            connection,
            db_path,
        })
    }

    /// Initialisiert die Migrations-Tabelle
    async fn init_migrations_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                table_name TEXT PRIMARY KEY,
                version INTEGER NOT NULL,
                applied_at INTEGER NOT NULL
            );",
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Returns a reference to the connection
    pub fn connection(&self) -> &SqlitePool {
        &self.connection
    }

    /// Returns the path to the database
    pub fn path(&self) -> &Path {
        &self.db_path
    }

    /// Returns the current migration version for a table
    async fn get_migration_version(&self, table_name: &str) -> Result<u32> {
        let version: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM schema_migrations WHERE table_name = ?",
        )
        .bind(table_name)
        .fetch_optional(&self.connection)
        .await?;

        Ok(version.unwrap_or(0) as u32)
    }

    /// Sets the migration version for a table
    async fn set_migration_version(&self, table_name: &str, version: u32) -> Result<()> {
        let timestamp = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT OR REPLACE INTO schema_migrations (table_name, version, applied_at) VALUES (?, ?, ?)",
        )
        .bind(table_name)
        .bind(version as i64)
        .bind(timestamp)
        .execute(&self.connection)
        .await?;
        Ok(())
    }

    /// Migrates a table for the specified DbEntity type
    /// Creates the table if it doesn't exist, or performs migrations
    pub async fn migrate_table<T: DbEntity>(&self) -> Result<()> {
        let table_name = T::table_name();
        let target_version = T::schema_version();
        let current_version = self.get_migration_version(table_name).await?;

        if current_version == 0 {
            // Table doesn't exist, create it
            T::create_table(&self.connection).await?;
            self.set_migration_version(table_name, target_version).await?;
        } else if current_version < target_version {
            // Migration erforderlich
            T::update_table(&self.connection, current_version, target_version).await?;
            self.set_migration_version(table_name, target_version).await?;
        }
        // current_version == target_version: nichts zu tun

        Ok(())
    }
}
