
use super::{Block, BlockEntry};
use db::DbEntity;
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use chrono::Utc;

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

    pub fn blocks(&self) -> &Vec<Block> {
        &self.blocks
    }

    pub fn append_entry(&mut self, entry: BlockEntry) -> Result<Block> {
        self.append_entries(vec![entry])
    }

    pub fn append_entries(&mut self, entries: Vec<BlockEntry>) -> Result<Block> {
        self.ensure_genesis();
        let block = self.build_block(entries)?;
        self.blocks.push(block);
        Ok(self.blocks.last().expect("block just pushed").clone())
    }

    pub async fn append_entries_with_db(
        &mut self,
        entries: Vec<BlockEntry>,
        database: &db::DB,
    ) -> Result<Block> {
        self.ensure_genesis();
        database.migrate_table::<Block>().await?;
        let block = self.build_block(entries)?;
        block.write(database.connection()).await?;
        self.blocks.push(block);
        Ok(self.blocks.last().expect("block just pushed").clone())
    }

    fn ensure_genesis(&mut self) {
        if self.blocks.is_empty() {
            self.blocks.push(Block::init());
        }
    }

    fn build_block(&self, data: Vec<BlockEntry>) -> Result<Block> {
        let prev = self
            .blocks
            .last()
            .ok_or_else(|| anyhow::anyhow!("Blockchain has no genesis block"))?;
        Ok(Block {
            version: prev.version(),
            header: super::block::Header {
                id: prev.id().inc(),
                prev_hash: prev.hash()?,
                timestamp: Utc::now().timestamp() as u64,
                links: Vec::new(),
            },
            data,
        })
    }
}


impl Blockchain{
    pub async fn from_db(database: &mut db::DB) -> Result<Self>{
        let mut blockchain = Self::new();
        database.migrate_table::<Block>().await?;
        let blocks = Block::list(database.connection()).await?;
        blockchain.blocks = blocks;
        Ok(blockchain)
    }
}