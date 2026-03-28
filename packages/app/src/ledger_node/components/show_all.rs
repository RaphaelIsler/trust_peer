use crate::components::{send_or_error, Errors};
use crate::i18n::{use_i18n, Key, LanguageSelector};
use crate::ToBackend;
use dioxus::prelude::*;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[component]
pub fn ShowAll() -> Element {
    let tx_backend = use_context::<mpsc::Sender<ToBackend>>();
    let ledger_nodes =
        use_context::<Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>>();
    let errors = use_context::<Errors>();
    let i18n = use_i18n();
    let mut show_create = use_signal(|| false);

    rsx! {
        div { class: "app-container",
            div { class: "app-header",
                h1 { class: "app-title", "{i18n.t(Key::AppTitle)}" }
                LanguageSelector {}
            }
            if ledger_nodes().is_empty() {
                crate::ledger_node::identification::Create {
                    on_create: move |ident: crate::ledger_node::Identification| {
                        send_or_error(&tx_backend, ToBackend::Create(ident), errors);
                        show_create.set(false);
                    },
                    on_error: move |_| {},
                }
            } else {
                for (private_str, store) in ledger_nodes() {
                    crate::ledger_node::frontend::Overview {
                        to_backend: {
                            let private_str = private_str.clone();
                            let tx = tx_backend.clone();
                            move |msg: crate::ledger_node::ToBackend| {
                                if let Ok(private) =
                                    blockchain::blockchain::Id::parse_str(&private_str)
                                {
                                    send_or_error(
                                        &tx,
                                        ToBackend::Ledger { private, msg },
                                        errors,
                                    );
                                }
                            }
                        },
                        on_show_peers: {
                            let private_str = private_str.clone();
                            let nav = navigator();
                            move |_| {
                                nav.push(crate::Route::PeerConnections {
                                    private_key: private_str.clone(),
                                });
                            }
                        },
                        store,
                    }
                }

                if show_create() {
                    crate::ledger_node::identification::Create {
                        on_create: move |ident: crate::ledger_node::Identification| {
                            send_or_error(&tx_backend, ToBackend::Create(ident), errors);
                            show_create.set(false);
                        },
                        on_error: move |_| {},
                    }
                } else {
                    button {
                        class: "btn btn--success",
                        onclick: move |_| show_create.set(true),
                        "{i18n.t(Key::AddIdentity)}"
                    }
                }
            }
        }
    }
}
