use crypto::{Salt, Signature};
use anyhow::Result;
use super::block_link::BlockLink;
use super::block_link::BlockLinkView;
use crate::block::hex_preview;
use dioxus::prelude::*;


pub type Id = helper::UId<BlockEntry>;


#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlockEntry{
    Verification{salt: Salt, public: crypto::PublicKey, signature: Signature},
    Identification{data: Vec<u8>},
    Link(BlockLink),
}


impl BlockEntry{
    pub fn new_verification(key: &crypto::KeyMeta) -> Result<Self> {
        let salt = Salt::new();
        let signature = Signature::new(&key.private_key, &salt.0)?;
        Ok(Self::Verification {
            salt,
            public: key.public_key.clone(),
            signature,
        })
    }

    pub fn from_link(link: BlockLink) -> Result<Self> {
        Ok(Self::Link(link))
    }
}

#[component]
pub fn BlockEntryList(entries: Vec<BlockEntry>) -> Element {
    rsx! {
        div { class: "block-entry-list",
            if entries.is_empty() {
                div { class: "block-entry-list__empty", "No entries" }
            }
            for entry in entries {
                BlockEntryView { entry }
            }
        }
    }
}

#[component]
pub fn BlockEntryView(entry: BlockEntry) -> Element {
    match entry {
        BlockEntry::Verification { salt, public, signature } => {
            let salt_preview = hex_preview(&salt.0, 12);
            let public_preview = hex_preview(public.as_bytes(), 12);
            let signature_len = signature.as_bytes().len();

            rsx! {
                div { class: "block-entry block-entry--verification",
                    div { class: "block-entry__title", "Verification" }
                    div { class: "block-entry__row", "Salt: {salt_preview}" }
                    div { class: "block-entry__row", "Public: {public_preview}" }
                    div { class: "block-entry__row", "Signature bytes: {signature_len}" }
                }
            }
        }
        BlockEntry::Identification { data } => {
            let data_preview = hex_preview(&data, 12);
            let data_len = data.len();

            rsx! {
                div { class: "block-entry block-entry--identification",
                    div { class: "block-entry__title", "Identification" }
                    div { class: "block-entry__row", "Bytes: {data_len}" }
                    div { class: "block-entry__row", "Preview: {data_preview}" }
                }
            }
        }
        BlockEntry::Link(link) => {
            rsx! {
                div { class: "block-entry block-entry--link",
                    div { class: "block-entry__title", "Link" }
                    BlockLinkView { link }
                }
            }
        }
    }
}
