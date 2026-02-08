pub type Id = helper::UId<BlockLink>;
use rand::Rng;
use super::{Block};
use crypto::{Hash, Salt};
use db::DbEntity;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlockLink{
    id: Id,
    from_blockchain: super::blockchain::Id,
    to: super::blockchain::Id,
    block: super::block::Id,
    salt: Salt,
    hash: Hash
}

impl BlockLink{
    fn from_block(from_blockchain: super::blockchain::Id, to_blockchain: super::blockchain::Id, block: &Block) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen(); // 32 Bytes = 256 Bit

        let mut block_bytes: Vec<u8> = bincode::serialize(&block)?;
        block_bytes.extend(random_bytes);
        let hash = Hash::new(&block_bytes);
        Ok(Self {
            id: Id::new(),
            from_blockchain,
            to: to_blockchain,
            block: block.id().clone(),
            salt: Salt::from_bytes(&random_bytes),
            hash: hash,
        })
    }
}

impl DbEntity for BlockLink {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.id
    }
    fn schema_version() -> u32 {
        1
    }
    fn table_name() -> &'static str {
        "block_links"
    }

    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS block_links (
                id TEXT PRIMARY KEY,
                from_blockchain_id TEXT NOT NULL,
                to_blockchain_id TEXT NOT NULL,
                block_id TEXT NOT NULL,
                salt BLOB NOT NULL,
                hash BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(from_blockchain_id) REFERENCES blockchains(id),
                FOREIGN KEY(to_blockchain_id) REFERENCES blockchains(id),
                FOREIGN KEY(block_id) REFERENCES blocks(id)
            );",
        )
        .execute(conn)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_block_links_from ON block_links(from_blockchain_id);")
            .execute(conn)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_block_links_to ON block_links(to_blockchain_id);")
            .execute(conn)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_block_links_block ON block_links(block_id);")
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
        let from_blockchain = self.from_blockchain.clone();
        let to = self.to.clone();
        let block = self.block.clone();
        let salt = self.salt.0.to_vec();
        let hash = self.hash.0.to_vec();
        let created_at = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT OR REPLACE INTO block_links (id, from_blockchain_id, to_blockchain_id, block_id, salt, hash, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(from_blockchain)
        .bind(to)
        .bind(block)
        .bind(salt)
        .bind(hash)
        .bind(created_at)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();

        let row = sqlx::query(
            "SELECT id, from_blockchain_id, to_blockchain_id, block_id, salt, hash
             FROM block_links WHERE id = ?",
        )
        .bind(&id)
        .fetch_optional(conn)
        .await?;

        match row {
            Some(row) => {
                let id: Id = row.try_get(0)?;
                let from_blockchain: super::blockchain::Id = row.try_get(1)?;
                let to: super::blockchain::Id = row.try_get(2)?;
                let block: super::block::Id = row.try_get(3)?;
                let salt_vec: Vec<u8> = row.try_get(4)?;
                let hash_vec: Vec<u8> = row.try_get(5)?;

                let mut salt_bytes = [0u8; 32];
                salt_bytes.copy_from_slice(&salt_vec);
                let salt = Salt::from_bytes(&salt_bytes);

                let mut hash_bytes = [0u8; 32];
                hash_bytes.copy_from_slice(&hash_vec);
                let hash = Hash(hash_bytes);

                Ok(Some(BlockLink {
                    id,
                    from_blockchain,
                    to,
                    block,
                    salt,
                    hash,
                }))
            }
            None => Ok(None),
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM block_links WHERE id = ?")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query(
            "SELECT id, from_blockchain_id, to_blockchain_id, block_id, salt, hash
             FROM block_links ORDER BY created_at",
        )
        .fetch_all(conn)
        .await?;

        let mut links = Vec::with_capacity(rows.len());
        for row in rows {
            let id: Id = row.try_get(0)?;
            let from_blockchain: super::blockchain::Id = row.try_get(1)?;
            let to: super::blockchain::Id = row.try_get(2)?;
            let block: super::block::Id = row.try_get(3)?;
            let salt_vec: Vec<u8> = row.try_get(4)?;
            let hash_vec: Vec<u8> = row.try_get(5)?;

            let mut salt_bytes = [0u8; 32];
            salt_bytes.copy_from_slice(&salt_vec);
            let salt = Salt::from_bytes(&salt_bytes);

            let mut hash_bytes = [0u8; 32];
            hash_bytes.copy_from_slice(&hash_vec);
            let hash = Hash(hash_bytes);

            links.push(BlockLink {
                id,
                from_blockchain,
                to,
                block,
                salt,
                hash,
            });
        }

        Ok(links)
    }
}
