pub type Id = helper::UId<Block>;
use rand::Rng;
use super::{Block};
use crypto::{Hash, Salt};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlockLink{
    to: super::blockchain::Id,
    block: super::block::Id,
    salt: Salt,
    hash: Hash
}

impl BlockLink{
    fn from_block(blockhain: super::blockchain::Id, block: &Block) -> anyhow::Result<Self> {
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen(); // 32 Bytes = 256 Bit

        let mut block_bytes: Vec<u8> = bincode::serialize(&block)?;
        block_bytes.extend(random_bytes);
        let hash = Hash::new(&block_bytes);
        Ok(Self {
            to: blockhain,
            block: block.id().clone(),
            salt: Salt::from_bytes(&random_bytes),
            hash: hash,
        })
    }
}
