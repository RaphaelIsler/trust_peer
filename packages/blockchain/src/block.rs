#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlockIdMarker;
pub type Id = helper::I64Id<BlockIdMarker>;
use db::DbEntity;
use serde::de::DeserializeOwned;

use crypto::Hash;
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Header{
    pub id: Id,
    pub prev_hash: Hash,
    pub timestamp: u64,
}

impl Header{
    pub fn init() -> Self{
        Self{
            id: Id::new(0),
            prev_hash: Hash::empty(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        }
    }

    fn from_block<T>(block: &Block<T>) -> anyhow::Result<Self> where T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq{
        Ok(Self{
            id: block.id().inc(),
            prev_hash: block.hash()?,
            timestamp: chrono::Utc::now().timestamp() as u64,
        })
    }
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Block<T> {
    pub version: u16,
    pub header: Header,
    pub data: Vec<T>,
}

impl<T> Block<T>
where
    T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq,
{
    pub fn init() -> Self{
        Self{
            version: 1,
            header: Header::init(),
            data: Vec::new(),
        }
    }

    pub fn hash(&self) -> anyhow::Result<Hash> {
        let header_bytes = bincode::serialize(self)?;
        Ok(Hash::new(&header_bytes))
    }

    pub fn id(&self) -> &Id {
        &self.header.id
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn header(&self) -> &Header {
        &self.header
    }

    pub fn data(&self) -> &Vec<T> {
        &self.data
    }
}

impl<T> DbEntity for Block<T>
where
    T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq,
{
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.header.id
    }

    fn table_name() -> &'static str {
        "blocks"
    }

    fn schema_version() -> u32 {
        1
    }


    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY,
                version INTEGER NOT NULL,
                prev_hash BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                data BLOB NOT NULL
            );",
        )
        .execute(conn)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp);",
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn update_table(conn: &SqlitePool, from_version: u32, _to_version: u32) -> Result<()> {
        match from_version {
            0 => Self::create_table(conn).await,
            _ => Ok(()), // No migrations yet
        }
    }

    async fn write(&self, conn: &SqlitePool) -> Result<()> {
        let id = self.id().clone();
        let version = self.version as i64;
        let prev_hash = self.header.prev_hash.0.to_vec();
        let timestamp = self.header.timestamp as i64;
        let data = bincode::serialize(&self.data)?;

        sqlx::query(
                "INSERT OR REPLACE INTO blocks (id, version, prev_hash, timestamp, data)
               VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(version)
        .bind(prev_hash)
        .bind(timestamp)
        .bind(data)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();

        let row = sqlx::query(
            "SELECT version, prev_hash, timestamp, data FROM blocks WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            let version: i64 = row.try_get(0)?;
            let prev_hash: Vec<u8> = row.try_get(1)?;
            let timestamp: i64 = row.try_get(2)?;
            let data_bytes: Vec<u8> = row.try_get(3)?;
            let data: Vec<T> = bincode::deserialize(&data_bytes)?;

            let mut prev_hash_array = [0u8; 32];
            prev_hash_array.copy_from_slice(&prev_hash);

            Ok(Some(Block {
                version: version as u16,
                header: Header {
                    id: id.clone(),
                    prev_hash: Hash(prev_hash_array),
                    timestamp: timestamp as u64,
                },
                data,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM blocks WHERE id = ?")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query("SELECT id FROM blocks ORDER BY timestamp")
            .fetch_all(conn)
            .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let id: Id = row.try_get(0)?;
            results.push(id);
        }

        let mut blocks = Vec::new();
        for id in results {
            if let Some(block) = Self::read(conn, &id).await? {
                blocks.push(block);
            }
        }
        Ok(blocks)
    }
}

#[component]
pub fn BlockHeaderView(header: Header) -> Element {
    let prev_hash = hex_preview(&header.prev_hash.0, 12);

    rsx! {
        div { class: "block-header",
            div { class: "block-header__row",
                span { class: "block-header__label", "Timestamp" }
                span { class: "block-header__value", "{header.timestamp}" }
            }
            div { class: "block-header__row",
                span { class: "block-header__label", "Prev Hash" }
                span { class: "block-header__value", "{prev_hash}" }
            }
        }
    }
}

#[component]
pub fn BlockView<T>(block: Block<T>, index: usize) -> Element
where
    T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq + 'static,
{
    let header = block.header().clone();
    let entry_count = block.data().len();

    rsx! {
        section { class: "block",
            h3 { class: "block__title", "Block #{index}" }
            BlockHeaderView { header }
            div { class: "block__meta", "Entries: {entry_count}" }
        }
    }
}

pub(crate) fn hex_preview(bytes: &[u8], max_chars: usize) -> String {
    let hex = hex::encode(bytes);
    if hex.len() <= max_chars {
        return hex;
    }

    let mut out = String::with_capacity(max_chars + 1);
    out.push_str(&hex[..max_chars]);
    out.push('…');
    out
}
