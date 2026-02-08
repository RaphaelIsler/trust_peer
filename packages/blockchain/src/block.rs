pub type Id = helper::UId<Block>;
use crate::block_entry::BlockEntry;
use db::DbEntity;

use super::BlockLink;
use crypto::Hash;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Header{
    pub id: Id,
    pub prev: Option<Id>,
    pub prev_hash: Hash,
    pub timestamp: u64,
    pub links: Vec<BlockLink>,
}

impl Header{
    fn init() -> Self{
        Self{
            id: Id::new(),
            prev: None,
            prev_hash: Hash::default(),
            timestamp: chrono::Utc::now().timestamp() as u64,
            links: Vec::new(),
        }
    }

    fn from_block(block: &Block) -> anyhow::Result<Self>{
        Ok(Self{
            id: Id::new(),
            prev: Some(block.id().clone()),
            prev_hash: block.hash()?,
            timestamp: chrono::Utc::now().timestamp() as u64,
            links: Vec::new(),
        })
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Block{
    pub version: u16,
    pub header: Header,
    pub data: Vec<BlockEntry>,
}

impl Block{
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

    pub fn data(&self) -> &Vec<BlockEntry> {
        &self.data
    }
}

impl DbEntity for Block {
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
                id TEXT PRIMARY KEY,
                blockchain_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                prev_id TEXT,
                prev_hash BLOB NOT NULL,
                timestamp INTEGER NOT NULL,
                data BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(blockchain_id) REFERENCES blockchains(id)
            );",
        )
        .execute(conn)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_blocks_blockchain ON blocks(blockchain_id, timestamp);",
        )
        .execute(conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_blocks_prev ON blocks(prev_id);")
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
        let prev_id = self.header.prev.clone();
        let prev_hash = self.header.prev_hash.0.to_vec();
        let timestamp = self.header.timestamp as i64;
        let data = bincode::serialize(&self.data)?;
        let created_at = chrono::Utc::now().timestamp();

        // Blockchain ID needs to be passed separately - for now use empty string
        let blockchain_id = String::new();

        sqlx::query(
            "INSERT OR REPLACE INTO blocks (id, blockchain_id, version, prev_id, prev_hash, timestamp, data, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(blockchain_id)
        .bind(version)
        .bind(prev_id)
        .bind(prev_hash)
        .bind(timestamp)
        .bind(data)
        .bind(created_at)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();

        let row = sqlx::query(
            "SELECT version, prev_id, prev_hash, timestamp, data FROM blocks WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        if let Some(row) = row {
            let version: i64 = row.try_get(0)?;
            let prev_id: Option<Id> = row.try_get(1)?;
            let prev_hash: Vec<u8> = row.try_get(2)?;
            let timestamp: i64 = row.try_get(3)?;
            let data_bytes: Vec<u8> = row.try_get(4)?;
            let data: Vec<BlockEntry> = bincode::deserialize(&data_bytes)?;

            let mut prev_hash_array = [0u8; 32];
            prev_hash_array.copy_from_slice(&prev_hash);

            Ok(Some(Block {
                version: version as u16,
                header: Header {
                    id: id.clone(),
                    prev: prev_id,
                    prev_hash: Hash(prev_hash_array),
                    timestamp: timestamp as u64,
                    links: Vec::new(), // Links stored separately
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
