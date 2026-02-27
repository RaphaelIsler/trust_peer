
use super::{Block, BlockEntry};
use db::DbEntity;
use anyhow::Result;
use chrono::Utc;
use dioxus::prelude::*;

pub type Id = helper::UId<Blockchain>;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Blockchain{
    id: Id,
    blocks: Vec<Block>,
}

impl Blockchain{
    pub fn new() -> Self{
        Self{
            id: Id::new(),
            blocks: vec![],
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn blocks(&self) -> &Vec<Block> {
        &self.blocks
    }

    pub async fn block_from(&self, count: usize, start_at: Option<usize>) -> Vec<Block> {
        let start_index = start_at.unwrap_or(0);
        self.blocks
            .iter()
            .skip(start_index)
            .take(count)
            .cloned()
            .collect()
    }

    pub fn append_entry(&mut self, entry: BlockEntry) -> Result<Block> {
        self.append_entries(vec![entry])
    }

    pub fn append_entries(&mut self, entries: Vec<BlockEntry>) -> Result<Block> {
        let block = self.build_block(entries)?;
        self.blocks.push(block);
        Ok(self.blocks.last().expect("block just pushed").clone())
    }

    pub async fn append_entries_with_db(
        &mut self,
        entries: Vec<BlockEntry>,
        database: &db::DB,
    ) -> Result<Block> {
        database.migrate_table::<Block>().await?;
        let block = self.build_block(entries)?;
        block.write(database.connection()).await?;
        self.blocks.push(block);
        Ok(self.blocks.last().expect("block just pushed").clone())
    }

    fn build_block(&self, data: Vec<BlockEntry>) -> Result<Block> {
        if self.blocks.is_empty() {
            Ok(Block {
                version: 0,
                header: super::block::Header::init(),
                data,
            })
        } else {
            let prev = self
                .blocks
                .last().expect("checked blocks is not empty") ;
            Ok(Block {
                version: prev.version(),
                header: super::block::Header {
                    id: prev.id().inc(),
                    prev_hash: prev.hash()?,
                    timestamp: Utc::now().timestamp() as u64,
                },
                data,
            })
        }
    }
}

#[component]
pub fn BlockchainView(chain: Blockchain) -> Element {
    let blocks = chain.blocks().clone();
    let empty_view = if blocks.is_empty() {
        Some(rsx! {
            div { class: "blockchain__empty", "No blocks" }
        })
    } else {
        None
    };

    rsx! {
        section { class: "blockchain",
            h2 { class: "blockchain__title", "Blockchain" }
            {empty_view}
            for (index , block) in blocks.into_iter().enumerate() {
                super::block::BlockView { index, block }
            }
        }
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
