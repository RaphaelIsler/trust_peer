use anyhow::Result;
use core_types::Timestamp;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::Id;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustState {
    WaitForNameAccept,
}

impl TrustState {
    fn to_db(self) -> &'static str {
        match self {
            Self::WaitForNameAccept => "wait_for_name_accept",
        }
    }

    fn from_db(value: &str) -> Result<Self> {
        match value {
            "wait_for_name_accept" => Ok(Self::WaitForNameAccept),
            other => anyhow::bail!("Unknown trust_state '{}'.", other),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub id: Id,
    pub last_public_singleton_checked: Option<Timestamp>,
    pub last_validated_public_block_id: Option<blockchain::block::Id>,
    pub trust_state: TrustState,
    pub last_seen_at: Timestamp,
    pub private_chain_id: Id,
}

impl State {
    pub fn new(id: Id, private_chain_id: Id) -> Self {
        Self {
            id,
            last_public_singleton_checked: None,
            last_validated_public_block_id: None,
            trust_state: TrustState::WaitForNameAccept,
            last_seen_at: Timestamp::now(),
            private_chain_id,
        }
    }
}

impl db::DbEntity for State {
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
                trust_state TEXT NOT NULL,
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
        let id = self.id.clone();
        let last_public_singleton_checked = self
            .last_public_singleton_checked
            .map(|timestamp| i64::try_from(timestamp.raw()))
            .transpose()?;
        let last_validated_public_block_id = self.last_validated_public_block_id.clone();
        let trust_state = self.trust_state.to_db();
        let last_seen_at = i64::try_from(self.last_seen_at.raw())?;
        let private_chain_id = self.private_chain_id.clone();

        sqlx::query(
            "INSERT OR REPLACE INTO peer_connection_states (
                id,
                last_public_singleton_checked,
                last_validated_public_block_id,
                trust_state,
                last_seen_at,
                private_chain_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(id)
        .bind(last_public_singleton_checked)
        .bind(last_validated_public_block_id)
        .bind(trust_state)
        .bind(last_seen_at)
        .bind(private_chain_id)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let row = sqlx::query(
            "SELECT id, last_public_singleton_checked, last_validated_public_block_id, trust_state, last_seen_at, private_chain_id
             FROM peer_connection_states
             WHERE id = ?1",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            let last_public_singleton_checked = match row.try_get::<Option<i64>, _>(1)? {
                Some(timestamp) if timestamp >= 0 => Some(Timestamp::from_raw(timestamp as u64)),
                Some(timestamp) => {
                    anyhow::bail!(
                        "Invalid negative last_public_singleton_checked '{}' for peer connection state '{}'.",
                        timestamp,
                        id
                    )
                }
                None => None,
            };

            let raw_last_seen_at: i64 = row.try_get(4)?;
            if raw_last_seen_at < 0 {
                anyhow::bail!(
                    "Invalid negative last_seen_at '{}' for peer connection state '{}'.",
                    raw_last_seen_at,
                    id
                );
            }

            Ok(Some(Self {
                id: row.try_get(0)?,
                last_public_singleton_checked,
                last_validated_public_block_id: row.try_get(2)?,
                trust_state: TrustState::from_db(&row.try_get::<String, _>(3)?)?,
                last_seen_at: Timestamp::from_raw(raw_last_seen_at as u64),
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
            "SELECT id, last_public_singleton_checked, last_validated_public_block_id, trust_state, last_seen_at, private_chain_id
             FROM peer_connection_states
             ORDER BY id",
        )
        .fetch_all(conn)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let id: Id = row.try_get(0)?;

            let last_public_singleton_checked = match row.try_get::<Option<i64>, _>(1)? {
                Some(timestamp) if timestamp >= 0 => Some(Timestamp::from_raw(timestamp as u64)),
                Some(timestamp) => {
                    anyhow::bail!(
                        "Invalid negative last_public_singleton_checked '{}' for peer connection state '{}'.",
                        timestamp,
                        id
                    )
                }
                None => None,
            };

            let raw_last_seen_at: i64 = row.try_get(4)?;
            if raw_last_seen_at < 0 {
                anyhow::bail!(
                    "Invalid negative last_seen_at '{}' for peer connection state '{}'.",
                    raw_last_seen_at,
                    id
                );
            }

            items.push(Self {
                id,
                last_public_singleton_checked,
                last_validated_public_block_id: row.try_get(2)?,
                trust_state: TrustState::from_db(&row.try_get::<String, _>(3)?)?,
                last_seen_at: Timestamp::from_raw(raw_last_seen_at as u64),
                private_chain_id: row.try_get(5)?,
            });
        }

        Ok(items)
    }
}
