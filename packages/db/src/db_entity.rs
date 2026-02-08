use anyhow::Result;
use sqlx::SqlitePool;

/// Generic trait for database entities
pub trait DbEntity: Sized + serde::Serialize + serde::de::DeserializeOwned {
    /// Unique identifier type for this entity
    type Id: Clone;

    /// Get the entity's ID
    fn id(&self) -> &Self::Id;

    /// Table name in the database
    fn table_name() -> &'static str;

    /// Current schema version for this entity
    fn schema_version() -> u32;

    /// Create the table schema
    async fn create_table(conn: &SqlitePool) -> Result<()>;

    /// Update table schema (for migrations)
    async fn update_table(conn: &SqlitePool, from_version: u32, to_version: u32) -> Result<()>;

    /// Write entity to database
    async fn write(&self, conn: &SqlitePool) -> Result<()>;

    /// Read entity from database by ID
    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>>;

    /// Delete entity from database
    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()>;

    /// List all entities (use with caution for large tables)
    async fn list(conn: &SqlitePool) -> Result<Vec<Self>>;
}
