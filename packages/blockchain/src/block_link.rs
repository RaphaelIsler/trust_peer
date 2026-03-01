pub type Id = helper::UId<BlockLink>;
use rand::Rng;
use super::{Block};
use crypto::{Hash, Salt};
use crate::block::hex_preview;
use dioxus::prelude::*;
use serde::de::DeserializeOwned;

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct BlockLink{
    to: super::blockchain::Id,
    block: super::block::Id,
    salt: Salt,
    hash: Hash
}

impl BlockLink{
    pub fn from_block<T>(to_blockchain: &super::blockchain::Id, block: &Block<T>) -> anyhow::Result<Self>
    where
        T: serde::Serialize + DeserializeOwned + Clone + PartialEq + Eq,
    {
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; 32] = rng.gen(); // 32 Bytes = 256 Bit

        let mut block_bytes: Vec<u8> = bincode::serialize(&block)?;
        block_bytes.extend(random_bytes);
        let hash = Hash::new(&block_bytes);
        Ok(Self {
            to: to_blockchain.clone(),
            block: block.id().clone(),
            salt: Salt::from_bytes(&random_bytes),
            hash: hash,
        })
    }

    pub fn to(&self) -> &super::blockchain::Id {
        &self.to
    }

    pub fn block(&self) -> &super::block::Id {
        &self.block
    }

    pub fn salt(&self) -> &Salt {
        &self.salt
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }
}

#[component]
pub fn BlockLinkList(links: Vec<BlockLink>) -> Element {
    rsx! {
        div { class: "block-link-list",
            if links.is_empty() {
                div { class: "block-link-list__empty", "No links" }
            }
            for link in links {
                BlockLinkView { link }
            }
        }
    }
}

#[component]
pub fn BlockLinkView(link: BlockLink) -> Element {
    let hash_preview = hex_preview(&link.hash().0, 12);
    let salt_preview = hex_preview(&link.salt().0, 12);

    rsx! {
        div { class: "block-link",
            div { class: "block-link__row", "Hash: {hash_preview}" }
            div { class: "block-link__row", "Salt: {salt_preview}" }
            div { class: "block-link__row", "To Blockchain: {link.to().inner()}|{link.block().inner()}" }
        }
    }
}
