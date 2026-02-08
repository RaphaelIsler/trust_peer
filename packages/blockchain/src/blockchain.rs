
use super::Block;
use db::DbEntity;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub type Id = helper::UId<Blockchain>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Blockchain{
    id: Id,
    blocks: Vec<Block>,
}

impl Blockchain{
    pub fn new() -> Self{
        Self{
            id: Id::new(),
            blocks: vec![Block::init()],
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }
}

impl DbEntity for Blockchain {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn table_name() -> &'static str {
        "blockchains"
    }

    fn schema_version() -> u32 {
        1
    }
    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS blockchains (
                id TEXT PRIMARY KEY,
                name TEXT,
                created_at INTEGER NOT NULL
            );",
        )
        .execute(conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_blockchains_created ON blockchains(created_at);")
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn update_table(conn: &SqlitePool, from_version: u32, _to_version: u32) -> Result<()> {
        match from_version {
            0 => Self::create_table(conn).await,
            _ => Ok(()),
        }
    }

    async fn write(&self, conn: &SqlitePool) -> Result<()> {
        let id = self.id().clone();
        let created_at = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT OR REPLACE INTO blockchains (id, name, created_at) VALUES (?, ?, ?)",
        )
        .bind(id)
        .bind(Option::<String>::None)
        .bind(created_at)
        .execute(conn)
        .await?;

        // Write all blocks with blockchain_id
        for block in &self.blocks {
            Self::write_block_for_blockchain(conn, &self.id, block).await?;
        }

        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let id_query = id.clone();

        let exists: Option<i64> = sqlx::query_scalar(
            "SELECT 1 FROM blockchains WHERE id = ?",
        )
        .bind(id_query)
        .fetch_optional(conn)
        .await?;

        let exists = exists.is_some();

        if !exists {
            return Ok(None);
        }

        // Load all blocks for this blockchain
        let blocks = Self::load_blocks_for_blockchain(conn, &id).await?;

        Ok(Some(Blockchain {
            id,
            blocks,
        }))
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM blocks WHERE blockchain_id = ?")
            .bind(&id)
            .execute(conn)
            .await?;

        sqlx::query("DELETE FROM blockchains WHERE id = ?")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query("SELECT id FROM blockchains ORDER BY created_at")
            .fetch_all(conn)
            .await?;

        let mut ids = Vec::with_capacity(rows.len());
        for row in rows {
            let id: Id = row.try_get(0)?;
            ids.push(id);
        }

        let mut blockchains = Vec::new();
        for id in ids {
            if let Some(bc) = Self::read(conn, &id).await? {
                blockchains.push(bc);
            }
        }
        Ok(blockchains)
    }
}

impl Blockchain {
    async fn write_block_for_blockchain(conn: &SqlitePool, blockchain_id: &Id, block: &Block) -> Result<()> {
        let id = block.id().clone();
        let blockchain_id = blockchain_id.clone();
        let version = block.version() as i64;
        let prev_id = block.header().prev.clone();
        let prev_hash = block.header().prev_hash.0.to_vec();
        let timestamp = block.header().timestamp as i64;
        let data = bincode::serialize(block.data())?;
        let created_at = chrono::Utc::now().timestamp();

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

    async fn load_blocks_for_blockchain(conn: &SqlitePool, blockchain_id: &Id) -> Result<Vec<Block>> {
        let blockchain_id = blockchain_id.clone();

        let rows = sqlx::query(
            "SELECT id, version, prev_id, prev_hash, timestamp, data
             FROM blocks WHERE blockchain_id = ? ORDER BY timestamp",
        )
        .bind(blockchain_id)
        .fetch_all(conn)
        .await?;

        let mut block_data: Vec<(
            super::block::Id,
            i64,
            Option<super::block::Id>,
            Vec<u8>,
            i64,
            Vec<u8>,
        )> = Vec::with_capacity(rows.len());
        for row in rows {
            block_data.push((
                row.try_get::<super::block::Id, _>(0)?,
                row.try_get::<i64, _>(1)?,
                row.try_get::<Option<super::block::Id>, _>(2)?,
                row.try_get::<Vec<u8>, _>(3)?,
                row.try_get::<i64, _>(4)?,
                row.try_get::<Vec<u8>, _>(5)?,
            ));
        }

        let mut blocks = Vec::new();
        for (block_id, version, prev, prev_hash_vec, timestamp, data_bytes) in block_data {
            let data = bincode::deserialize(&data_bytes)?;

            let mut prev_hash = [0u8; 32];
            prev_hash.copy_from_slice(&prev_hash_vec);

            blocks.push(Block {
                version: version as u16,
                header: super::block::Header {
                    id: block_id,
                    prev,
                    prev_hash: crypto::Hash(prev_hash),
                    timestamp: timestamp as u64,
                    links: Vec::new(),
                },
                data,
            });
        }

        Ok(blocks)
    }
}
