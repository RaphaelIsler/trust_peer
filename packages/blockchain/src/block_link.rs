pub type Id = helper::UId<BlockLink>;
use rand::Rng;
use super::{Block};
use crypto::{Hash, Salt};
use db::DbEntity;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct BlockLink{
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
            from_blockchain,
            to: to_blockchain,
            block: block.id().clone(),
            salt: Salt::from_bytes(&random_bytes),
            hash: hash,
        })
    }
}
