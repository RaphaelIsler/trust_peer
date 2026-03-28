use crate::components::{send_or_error, Errors};
use crate::i18n::{use_i18n, Key};
use crate::ledger_node::ToBackend as LedgerToBackend;
use crate::peer_connection::{Connection, ConnectionEntry, ConnectionView, Id};
use crate::ToBackend;
use dioxus::prelude::*;
use std::collections::HashMap;
use tokio::sync::mpsc;

use super::new_connection::NewConnection;

#[derive(Clone, PartialEq)]
enum Mode {
    List,
    NewConnection,
}

#[cfg(feature = "frontend")]
#[component]
pub fn ShowAll(private_key: String) -> Element {
    let tx_backend = use_context::<mpsc::Sender<ToBackend>>();
    let ledger_nodes =
        use_context::<Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>>();
    let errors = use_context::<Errors>();
    let i18n = use_i18n();
    let mut mode = use_signal(|| Mode::List);

    let private_key_effect = private_key.clone();
    let tx_effect = tx_backend.clone();
    use_effect(move || {
        if let Ok(private) = blockchain::blockchain::Id::parse_str(&private_key_effect) {
            send_or_error(
                &tx_effect,
                ToBackend::Ledger {
                    private,
                    msg: LedgerToBackend::GetPeerConnections,
                },
                errors,
            );
        }
    });

    let active_connections: Vec<Connection> = ledger_nodes()
        .get(&private_key)
        .map(|f| {
            f.connections_full
                .iter()
                .filter_map(|e| {
                    if let ConnectionEntry::Active(c) = e {
                        Some(c.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let nav = navigator();

    rsx! {
        div { class: "peer-connections-page",
            div { class: "peer-connections-page__header",
                button {
                    class: "btn btn--secondary btn--sm",
                    onclick: move |_| {
                        if mode() == Mode::List {
                            nav.go_back();
                        } else {
                            mode.set(Mode::List);
                        }
                    },
                    "{i18n.t(Key::Back)}"
                }
                if mode() == Mode::List {
                    button {
                        class: "btn btn--primary btn--sm",
                        onclick: move |_| mode.set(Mode::NewConnection),
                        "{i18n.t(Key::NewConnection)}"
                    }
                }
            }

            match mode() {
                Mode::List => rsx! {
                    if active_connections.is_empty() {
                        p { class: "peer-connections__empty", "{i18n.t(Key::NoConnections)}" }
                    } else {
                        div { class: "peer-connections",
                            for connection in active_connections {
                                ConnectionView {
                                    key: "{connection.id.inner()}",
                                    connection: connection.clone(),
                                    on_save_name: {
                                        let tx = tx_backend.clone();
                                        let private_key = private_key.clone();
                                        move |(id, name): (Id, Option<String>)| {
                                            if let Ok(private) =
                                                blockchain::blockchain::Id::parse_str(&private_key)
                                            {
                                                send_or_error(
                                                    &tx,
                                                    ToBackend::Ledger {
                                                        private,
                                                        msg: LedgerToBackend::UpdatePeerConnectionName {
                                                            id,
                                                            name,
                                                        },
                                                    },
                                                    errors,
                                                );
                                            }
                                        }
                                    },
                                }
                            }
                        }
                    }
                },
                Mode::NewConnection => rsx! {
                    NewConnection {
                        private_key: private_key.clone(),
                        on_done: move |_| mode.set(Mode::List),
                    }
                },
            }
        }
    }
}
