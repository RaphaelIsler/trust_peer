use super::{identification::Identification, ToBackend, ToFrontend};
use crate::AppBlock;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontend {
    pub current: crate::money::Money,
    pub private: blockchain::blockchain::Id,
    pub public: blockchain::blockchain::Id,
    pub identifications: Vec<Identification>,
    pub active_chain: Option<(blockchain::blockchain::Id, Vec<AppBlock>)>,
}

impl Frontend {
    pub fn new() -> Self {
        Self {
            current: crate::money::Money::default(),
            private: blockchain::blockchain::Id::new(),
            public: blockchain::blockchain::Id::new(),
            identifications: vec![],
            active_chain: None,
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
            } => {
                self.private = private;
                self.public = public;
                self.current = money;
                self.identifications = identifications;
            }
            ToFrontend::Identifications(identifications) => {
                self.identifications = identifications;
            }
            ToFrontend::Blocks { id, blocks } => {
                self.active_chain = Some((id, blocks));
            }
            _ => {}
        }
    }
}

#[cfg(feature = "frontend")]
#[path = "."]
mod m_frontend {
    use super::*;
    #[component]
    pub fn Overview(to_backend: EventHandler<ToBackend>, store: super::Frontend) -> Element {
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
            div {
                class: "ledger-node",
                style: "border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 8px; background: white;",

                div { style: "display: flex; justify-content: space-between; align-items: center;",
                    h3 { style: "margin: 0;", "{store.display_name()}" }
                    crate::money::MoneyView {
                        money: store.current,
                        shown_amount: crate::money::MoneyViewMode::Current,
                    }

                    button {
                        onclick: move |_| show_details.set(!show_details()),
                        style: "padding: 5px 15px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer;",
                        if show_details() {
                            "Less"
                        } else {
                            "Details"
                        }
                    }
                }

                if show_details() {
                    div { style: "margin-top: 15px; padding-top: 15px; border-top: 1px solid #eee;",
                        div { style: "margin-top: 10px;",
                            strong { "Private chain: " }
                            span { style: "font-family: monospace; font-size: 0.85em;",
                                "{store.private.to_string()}"
                            }
                        }
                        div { style: "margin-top: 6px;",
                            strong { "Public chain: " }
                            span { style: "font-family: monospace; font-size: 0.85em;",
                                "{store.public.to_string()}"
                            }
                        }
                        if !store.identifications.is_empty() {
                            div { style: "margin-top: 10px;",
                                strong { "Identifications:" }
                                for ident in store.identifications.iter() {
                                    crate::ledger_node::identification::Show { identification: ident.clone() }
                                }
                            }
                        }

                        // Chain viewer
                        div { style: "margin-top: 16px; display: flex; gap: 8px;",
                            button {
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
                                style: "padding: 6px 12px; background: #0d6efd; color: white; border: none; border-radius: 4px; cursor: pointer;",
                                "Private Chain"
                            }
                            button {
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
                                style: "padding: 6px 12px; background: #6610f2; color: white; border: none; border-radius: 4px; cursor: pointer;",
                                "Public Chain"
                            }
                        }

                        if chain_loading {
                            div { style: "margin-top: 10px; color: #666;", "Loading blockchain..." }
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
