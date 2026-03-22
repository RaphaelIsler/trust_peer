use anyhow::Result;
use core_types::Timestamp;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::Id;



#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustState {
    pub id: Id,
    pub last_public_singleton_checked: Option<Timestamp>,
    pub last_validated_public_block_id: Option<blockchain::block::Id>,
    pub name_accepted: Option<Timestamp>,
    pub last_seen_at: Timestamp,
    pub private_chain_id: Id,
}

impl TrustState {
    pub fn new(id: Id, private_chain_id: Id) -> Self {
        Self {
            id,
            last_public_singleton_checked: None,
            last_validated_public_block_id: None,
            name_accepted: None,
            last_seen_at: Timestamp::now(),
            private_chain_id,
        }
    }
}

impl db::DbEntity for TrustState {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn table_name() -> &'static str {
        "peer_connection_states"
    }

    fn schema_version() -> u32 {
        1
    }

    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS peer_connection_states (
                id TEXT PRIMARY KEY REFERENCES user_connections(id) ON DELETE CASCADE,
                last_public_singleton_checked INTEGER,
                last_validated_public_block_id INTEGER,
                name_accepted INTEGER,
                last_seen_at INTEGER NOT NULL,
                private_chain_id TEXT NOT NULL
            );",
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn update_table(
        _conn: &SqlitePool,
        _from_version: u32,
        _to_version: u32,
    ) -> Result<()> {
        Ok(())
    }

    async fn write(&self, conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO peer_connection_states (
                id,
                last_public_singleton_checked,
                last_validated_public_block_id,
                name_accepted,
                last_seen_at,
                private_chain_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(self.id.clone())
        .bind(self.last_public_singleton_checked)
        .bind(self.last_validated_public_block_id.clone())
        .bind(self.name_accepted)
        .bind(self.last_seen_at)
        .bind(self.private_chain_id.clone())
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let row = sqlx::query(
            "SELECT id, last_public_singleton_checked, last_validated_public_block_id, name_accepted, last_seen_at, private_chain_id
             FROM peer_connection_states
             WHERE id = ?1",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            Ok(Some(Self {
                id: row.try_get(0)?,
                last_public_singleton_checked: row.try_get(1)?,
                last_validated_public_block_id: row.try_get(2)?,
                name_accepted: row.try_get(3)?,
                last_seen_at: row.try_get(4)?,
                private_chain_id: row.try_get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM peer_connection_states WHERE id = ?1")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query(
            "SELECT id, last_public_singleton_checked, last_validated_public_block_id, name_accepted, last_seen_at, private_chain_id
             FROM peer_connection_states
             ORDER BY id",
        )
        .fetch_all(conn)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(Self {
                id: row.try_get(0)?,
                last_public_singleton_checked: row.try_get(1)?,
                last_validated_public_block_id: row.try_get(2)?,
                name_accepted: row.try_get(3)?,
                last_seen_at: row.try_get(4)?,
                private_chain_id: row.try_get(5)?,
            });
        }

        Ok(items)
    }
}
