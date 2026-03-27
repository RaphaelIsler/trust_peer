use crate::i18n::{use_i18n, Key};
use crate::money::{Money, MoneyView, MoneyViewMode};
use anyhow::Result;
use blockchain::{Block, BlockHeaderView, BlockLink, BlockLinkView};
use core_types::TimestampView;
use crypto::{Salt, Signature};
use dioxus::prelude::*;

pub type AppBlock = Block<BlockEntry>;

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BlockEntry {
    Verification {
        salt: Salt,
        public: crypto::PublicKey,
        signature: Signature,
    },
    Identification {
        data: Vec<u8>,
    },
    Link(BlockLink),
    CurrentAmount {
        money: Money,
    },
}

impl BlockEntry {
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
    let i18n = use_i18n();
    rsx! {
        div { class: "block-entry-list",
            if entries.is_empty() {
                div { class: "block-entry-list__empty", "{i18n.t(Key::NoEntries)}" }
            }
            for entry in entries {
                BlockEntryView { entry }
            }
        }
    }
}

#[component]
pub fn BlockEntryView(entry: BlockEntry) -> Element {
    let i18n = use_i18n();
    match entry {
        BlockEntry::Verification {
            salt,
            public,
            signature,
        } => {
            let salt_preview = hex_preview(&salt.0, 12);
            let public_preview = hex_preview(public.as_bytes(), 12);
            let signature_len = signature.as_bytes().len();

            rsx! {
                div { class: "block-entry block-entry--verification",
                    div { class: "block-entry__title", "{i18n.t(Key::EntryVerification)}" }
                    div { class: "block-entry__row", "{i18n.t(Key::Salt)} {salt_preview}" }
                    div { class: "block-entry__row", "{i18n.t(Key::PublicKey)} {public_preview}" }
                    div { class: "block-entry__row", "{i18n.t(Key::SignatureBytes)} {signature_len}" }
                }
            }
        }
        BlockEntry::Identification { data } => {
            let data_preview = hex_preview(&data, 12);
            let data_len = data.len();

            rsx! {
                div { class: "block-entry block-entry--identification",
                    div { class: "block-entry__title", "{i18n.t(Key::EntryIdentification)}" }
                    div { class: "block-entry__row", "{i18n.t(Key::Bytes)} {data_len}" }
                    div { class: "block-entry__row", "{i18n.t(Key::Preview)} {data_preview}" }
                }
            }
        }
        BlockEntry::Link(link) => {
            rsx! {
                div { class: "block-entry block-entry--link",
                    div { class: "block-entry__title", "{i18n.t(Key::EntryLink)}" }
                    BlockLinkView { link }
                }
            }
        }
        BlockEntry::CurrentAmount { money } => {
            let timestamp = money.timestamp();
            rsx! {
                div { class: "block-entry block-entry--current-amount",
                    div { class: "block-entry__title", "{i18n.t(Key::EntryCurrentAmount)}" }
                    div { class: "block-entry__row",
                        "{i18n.t(Key::Timestamp)} "
                        TimestampView { timestamp }
                    }
                    div { class: "block-entry__row",
                        "{i18n.t(Key::Amount)} "
                        MoneyView { money, shown_amount: MoneyViewMode::Stored }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BlockView(block: AppBlock, index: usize) -> Element {
    let i18n = use_i18n();
    let header = block.header().clone();
    let entries = block.data().clone();
    let entry_count = entries.len();

    rsx! {
        section { class: "block",
            h3 { class: "block__title", "{i18n.t(Key::Block)} #{index}" }
            BlockHeaderView { header }
            div { class: "block__meta", "{i18n.t(Key::Entries)} {entry_count}" }
            BlockEntryList { entries }
        }
    }
}

/// Renders a full blockchain from an `Option<Vec<AppBlock>>`.
/// Shows nothing when `None`, an empty-state message when the vec is empty,
/// and the block list otherwise.
#[component]
pub fn BlockChainView(blocks: Option<Vec<AppBlock>>) -> Element {
    let i18n = use_i18n();
    let Some(blocks) = blocks else {
        return rsx! {};
    };

    rsx! {
        div { class: "blockchain",
            if blocks.is_empty() {
                div { class: "blockchain__empty", "{i18n.t(Key::NoBlocks)}" }
            }
            for (index , block) in blocks.into_iter().enumerate() {
                BlockView { index, block }
            }
        }
    }
}

fn hex_preview(bytes: &[u8], max_chars: usize) -> String {
    let hex = hex::encode(bytes);
    if hex.len() <= max_chars {
        return hex;
    }

    let mut out = String::with_capacity(max_chars + 1);
    out.push_str(&hex[..max_chars]);
    out.push('…');
    out
}
