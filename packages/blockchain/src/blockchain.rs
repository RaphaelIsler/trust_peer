
use super::Block;
use db::DbEntity;
use anyhow::Result;
use chrono::Utc;
use crypto::Hash;
use dioxus::prelude::*;
use serde::de::DeserializeOwned;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlockchainIdMarker;
pub type Id = helper::UId<BlockchainIdMarker>;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Blockchain<T> {
    id: Id,
    blocks: Vec<Block<T>>,
}

impl<T> Blockchain<T>
where
    T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq,
{
    pub fn new() -> Self{
        Self{
            id: Id::new(),
            blocks: vec![],
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn blocks(&self) -> &Vec<Block<T>> {
        &self.blocks
    }

    /// Iterates all entries from oldest to newest across all blocks.
    pub fn entries(&self) -> impl Iterator<Item = &T> {
        self.blocks.iter().flat_map(|block| block.data().iter())
    }

    /// Finds the newest mapped entry by scanning from the back and stopping at first match.
    pub fn last_entry_map<U, F>(&self, mapper: F) -> Option<U>
    where
        F: FnMut(&T) -> Option<U>,
    {
        self.blocks
            .iter()
            .rev()
            .flat_map(|block| block.data().iter().rev())
            .find_map(mapper)
    }

    /// Finds the oldest mapped entry by scanning from the front and stopping at first match.
    pub fn first_entry_map<U, F>(&self, mapper: F) -> Option<U>
    where
        F: FnMut(&T) -> Option<U>,
    {
        self.entries().find_map(mapper)
    }

    pub async fn block_from(&self, count: usize, start_at: Option<usize>) -> Vec<Block<T>> {
        let start_index = start_at.unwrap_or(0);
        self.blocks
            .iter()
            .skip(start_index)
            .take(count)
            .cloned()
            .collect()
    }

    pub fn append_entry(&mut self, entry: T) -> Result<Block<T>> {
        self.append_entries(vec![entry])
    }

    pub fn append_entries(&mut self, entries: Vec<T>) -> Result<Block<T>> {
        let block = self.build_block(entries)?;
        self.blocks.push(block);
        Ok(self.blocks.last().expect("block just pushed").clone())
    }

    pub async fn append_entries_with_db(
        &mut self,
        entries: Vec<T>,
        database: &db::DB,
    ) -> Result<Block<T>> {
        database.migrate_table::<Block<T>>().await?;
        let block = self.build_block(entries)?;
        block.write(database.connection()).await?;
        self.blocks.push(block);
        Ok(self.blocks.last().expect("block just pushed").clone())
    }

    fn build_block(&self, data: Vec<T>) -> Result<Block<T>> {
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

    pub fn verify_with_state<S, F>(&self, mut state: S, mut apply: F) -> Result<S>
    where
        F: FnMut(&mut S, &T) -> Result<()>,
    {
        let mut previous_block: Option<&Block<T>> = None;

        for (block_index, block) in self.blocks.iter().enumerate() {
            match previous_block {
                None => {
                    if block.id() != &super::block::Id::new(0) {
                        anyhow::bail!(
                            "invalid genesis block id at block index {block_index}: expected 0"
                        );
                    }

                    if block.header().prev_hash != Hash::empty() {
                        anyhow::bail!(
                            "invalid genesis prev_hash at block index {block_index}: expected empty hash"
                        );
                    }
                }
                Some(prev) => {
                    let expected_id = prev.id().inc();
                    if block.id() != &expected_id {
                        anyhow::bail!(
                            "invalid block id at block index {block_index}: expected sequential id"
                        );
                    }

                    let expected_prev_hash = prev.hash()?;
                    if block.header().prev_hash != expected_prev_hash {
                        anyhow::bail!(
                            "invalid prev_hash at block index {block_index}: chain linkage mismatch"
                        );
                    }

                    if block.header().timestamp < prev.header().timestamp {
                        anyhow::bail!(
                            "invalid timestamp at block index {block_index}: timestamp moved backwards"
                        );
                    }
                }
            }

            for (entry_index, entry) in block.data().iter().enumerate() {
                apply(&mut state, entry).map_err(|error| {
                    anyhow::anyhow!(
                        "state verification failed at block index {block_index}, entry index {entry_index}: {error}"
                    )
                })?;
            }

            previous_block = Some(block);
        }

        Ok(state)
    }
}

#[component]
pub fn BlockchainView<T>(chain: Blockchain<T>) -> Element
where
    T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq + 'static,
{
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


impl<T> Blockchain<T>
where
    T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq,
{
    pub async fn from_db(database: &mut db::DB) -> Result<Self>{
        let mut blockchain = Self::new();
        database.migrate_table::<Block<T>>().await?;
        let blocks = Block::<T>::list(database.connection()).await?;
        blockchain.blocks = blocks;
        Ok(blockchain)
    }
}
