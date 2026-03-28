use super::{identification::Identification, ToBackend, ToFrontend};
use crate::{AppBlock, peer_connection};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontend {
    pub current: crate::money::Money,
    pub private: blockchain::blockchain::Id,
    pub public: blockchain::blockchain::Id,
    pub identifications: Vec<Identification>,
    pub active_chain: Option<(blockchain::blockchain::Id, Vec<AppBlock>)>,
    pub connections: peer_connection::compact::Store,
    pub connections_full: Vec<peer_connection::ConnectionEntry>,
}

impl Frontend {
    pub fn new() -> Self {
        Self {
            current: crate::money::Money::default(),
            private: blockchain::blockchain::Id::new(),
            public: blockchain::blockchain::Id::new(),
            identifications: vec![],
            active_chain: None,
            connections: peer_connection::compact::Store::default(),
            connections_full: vec![],
        }
    }

    pub fn display_name(&self) -> String {
        self.identifications
            .first()
            .map(|i| i.display_name())
            .unwrap_or_else(|| self.private.to_string())
    }

    pub fn handle_msg(&mut self, msg: ToFrontend) {
        match msg {
            ToFrontend::Initialized {
                private,
                public,
                money,
                identifications,
                connection,
            } => {
                self.private = private;
                self.public = public;
                self.current = money;
                self.identifications = identifications;
                self.connections = connection;
            }
            ToFrontend::Identifications(identifications) => {
                self.identifications = identifications;
            }
            ToFrontend::Blocks { id, blocks } => {
                self.active_chain = Some((id, blocks));
            }
            ToFrontend::PeerConnections { connections } => {
                // Refresh Active entries; keep any Pending/Failed entries intact.
                self.connections_full
                    .retain(|e| !matches!(e, peer_connection::ConnectionEntry::Active(_)));
                for c in connections {
                    self.connections_full
                        .push(peer_connection::ConnectionEntry::Active(c));
                }
            }
            ToFrontend::ConnectionsIds { new_connection_id, ids } => {
                // Backend generated IDs for the initiating side — store them in the
                // matching Pending entry (or create one if not yet present).
                let entry = self.connections_full.iter_mut().find(|e| {
                    e.new_connection_id() == Some(new_connection_id)
                });
                if let Some(peer_connection::ConnectionEntry::Pending { ids: slot, .. }) = entry {
                    *slot = Some(ids);
                } else {
                    self.connections_full.push(peer_connection::ConnectionEntry::Pending {
                        new_connection_id,
                        ids: Some(ids),
                        progress: None,
                    });
                }
            }
            ToFrontend::ConnectionState { new_connection_id, current_working, percentage } => {
                let entry = self.connections_full.iter_mut().find(|e| {
                    e.new_connection_id() == Some(new_connection_id)
                });
                if let Some(peer_connection::ConnectionEntry::Pending { progress, .. }) = entry {
                    *progress = Some((current_working, percentage));
                } else {
                    // Backend sent state before ConnectionsIds — create the entry.
                    self.connections_full.push(peer_connection::ConnectionEntry::Pending {
                        new_connection_id,
                        ids: None,
                        progress: Some((current_working, percentage)),
                    });
                }
            }
            ToFrontend::ConnectionEstablished { new_connection_id, .. } => {
                // Remove the Pending entry; the Active one arrives via PeerConnections.
                self.connections_full
                    .retain(|e| e.new_connection_id() != Some(new_connection_id));
            }
            ToFrontend::ConnectionEstablishedFailed { new_connection_id } => {
                if let Some(entry) = self.connections_full.iter_mut().find(|e| {
                    e.new_connection_id() == Some(new_connection_id)
                }) {
                    *entry = peer_connection::ConnectionEntry::Failed { new_connection_id };
                }
            }
            _ => {}
        }
    }
}

#[cfg(feature = "frontend")]
#[path = "."]
mod m_frontend {
    use super::*;
    use crate::i18n::{use_i18n, Key};
    #[component]
    pub fn Overview(
        to_backend: EventHandler<ToBackend>,
        store: super::Frontend,
        on_show_peers: EventHandler<()>,
    ) -> Element {
        let i18n = use_i18n();
        let mut show_details = use_signal(|| false);
        let mut selected_chain =
            use_signal(|| None::<blockchain::blockchain::Id>);

        // Load all init data in one shot on first render.
        use_effect(move || {
            to_backend.call(ToBackend::Init);
        });

        // True while the selected chain hasn't arrived in the store yet.
        let chain_loading = selected_chain().is_some()
            && store
                .active_chain
                .as_ref()
                .map(|(id, _)| id.clone())
                != selected_chain();

        // Which blocks to display: only when the store's chain matches the selection.
        let displayed_blocks = match (&store.active_chain, selected_chain()) {
            (Some((id, blocks)), Some(sel)) if *id == sel => Some(blocks.clone()),
            _ => None,
        };

        rsx! {
            div { class: "ledger-node",
                div { class: "ledger-node__header",
                    h3 { class: "ledger-node__name", "{store.display_name()}" }
                    crate::money::MoneyView {
                        money: store.current,
                        shown_amount: crate::money::MoneyViewMode::Current,
                    }
                    button {
                        class: "btn btn--secondary btn--sm",
                        onclick: move |_| show_details.set(!show_details()),
                        if show_details() {
                            "{i18n.t(Key::Less)}"
                        } else {
                            "{i18n.t(Key::Details)}"
                        }
                    }
                }

                button { onclick: move |_| on_show_peers.call(()),
                    crate::peer_connection::compact::Compact { store: store.connections.clone() }
                }

                if show_details() {
                    div { class: "ledger-node__details",
                        div { class: "ledger-node__chain-info",
                            strong { "{i18n.t(Key::PrivateChainLabel)}" }
                            span { class: "ledger-node__chain-id", "{store.private}" }
                        }
                        div { class: "ledger-node__chain-info",
                            strong { "{i18n.t(Key::PublicChainLabel)}" }
                            span { class: "ledger-node__chain-id", "{store.public}" }
                        }

                        if !store.identifications.is_empty() {
                            div { class: "ledger-node__identifications",
                                strong { "{i18n.t(Key::Identifications)}" }
                                for ident in store.identifications.iter() {
                                    crate::ledger_node::identification::Show { identification: ident.clone() }
                                }
                            }
                        }

                        div { class: "ledger-node__chain-actions",
                            button {
                                class: "btn btn--primary btn--sm",
                                onclick: {
                                    let private_id = store.private.clone();
                                    move |_| {
                                        selected_chain.set(Some(private_id.clone()));
                                        to_backend
                                            .call(ToBackend::GetBlocks {
                                                id: private_id.clone(),
                                                count: 100,
                                                start_at: None,
                                            });
                                    }
                                },
                                "{i18n.t(Key::PrivateChainBtn)}"
                            }
                            button {
                                class: "btn btn--purple btn--sm",
                                onclick: {
                                    let public_id = store.public.clone();
                                    move |_| {
                                        selected_chain.set(Some(public_id.clone()));
                                        to_backend
                                            .call(ToBackend::GetBlocks {
                                                id: public_id.clone(),
                                                count: 100,
                                                start_at: None,
                                            });
                                    }
                                },
                                "{i18n.t(Key::PublicChainBtn)}"
                            }
                        }

                        if chain_loading {
                            div { class: "ledger-node__loading", "{i18n.t(Key::LoadingBlockchain)}" }
                        }

                        crate::block_entry::BlockChainView { blocks: displayed_blocks }
                    }
                }
            }
        }
    }
}
#[cfg(feature = "frontend")]
#[allow(unused_imports)]
pub use m_frontend::*;
pub use m_frontend::*;
