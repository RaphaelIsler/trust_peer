use crypto::{Salt, Signature};
use db::DbEntity;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub type Id = helper::UId<BlockEntry>;


#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub enum BlockEntry{
    Verification{salt: Salt, public: Vec<u8>, signature: Signature},
    Identification{data: Vec<u8>},
}

impl DbEntity for BlockEntry {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        // BlockEntry doesn't have its own ID - it's part of a Block's data vector
        // This is a design decision: entries are stored serialized inside blocks
        // If you need individual entry IDs, add an id field to each variant
        unimplemented!("BlockEntry is stored as part of Block data, not as separate rows")
    }

    fn table_name() -> &'static str {
        "block_entries"
    }

    fn schema_version() -> u32 {
        1
    }

    async fn create_table(conn: &SqlitePool) -> Result<()> {
        // BlockEntries are stored serialized within Block.data as a Vec<BlockEntry>
        // No separate table needed - this is just for interface completeness
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS block_entries (
                id TEXT PRIMARY KEY,
                block_id TEXT NOT NULL,
                entry_type TEXT NOT NULL,
                entry_data BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(block_id) REFERENCES blocks(id)
            );",
        )
        .execute(conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_block_entries_block ON block_entries(block_id);")
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

    async fn write(&self, _conn: &SqlitePool) -> Result<()> {
        // BlockEntries are written as part of Block::write()
        // They are serialized into Block.data field
        unimplemented!("BlockEntry is stored as part of Block data")
    }

    async fn read(_conn: &SqlitePool, _id: &Self::Id) -> Result<Option<Self>> {
        // BlockEntries are read as part of Block::read()
        unimplemented!("BlockEntry is stored as part of Block data")
    }

    async fn delete(_conn: &SqlitePool, _id: &Self::Id) -> Result<()> {
        // BlockEntries are deleted with their parent Block
        unimplemented!("BlockEntry is stored as part of Block data")
    }

    async fn list(_conn: &SqlitePool) -> Result<Vec<Self>> {
        // BlockEntries are listed via their parent Block
        unimplemented!("BlockEntry is stored as part of Block data")
    }
}

impl BlockEntry {
    /// Helper to query all entries for a specific block
    pub async fn list_for_block(conn: &SqlitePool, block_id: &crate::block::Id) -> Result<Vec<Self>> {
        let block_id = block_id.clone();

        let rows = sqlx::query(
            "SELECT entry_data FROM block_entries WHERE block_id = ? ORDER BY created_at",
        )
        .bind(block_id)
        .fetch_all(conn)
        .await?;

        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            let data: Vec<u8> = row.try_get(0)?;
            results.push(data);
        }

        let mut entries = Vec::new();
        for data in results {
            let entry: BlockEntry = bincode::deserialize(&data)?;
            entries.push(entry);
        }

        Ok(entries)
    }
}
