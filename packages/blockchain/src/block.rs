pub type Id = helper::UId<Block>;
use crate::block_entry::BlockEntry;

use super::BlockLink;
use crypto::Hash;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Header{
    id: Id,
    prev: Option<Id>,
    prev_hash: Hash,
    timestamp: u64,
    links: Vec<BlockLink>,
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
    version: u16,
    header: Header,
    data: Vec<BlockEntry>,
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
}
