use crate::components::{send_or_error, BarcodeDisplay, BarcodeReader, Errors};
use crate::i18n::{use_i18n, Key};
use crate::ledger_node::ToBackend as LedgerToBackend;
use crate::peer_connection::{ConnectionEntry, WebRtcIds, WebRtcIdsInputView, WebRtcIdsShareView};
use crate::ToBackend;
use dioxus::prelude::*;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Clone, PartialEq)]
enum Flow {
    Choosing,
    Initiating { new_connection_id: u8 },
    Responding { new_connection_id: u8 },
}

#[cfg(feature = "frontend")]
#[component]
pub fn NewConnection(private_key: String, on_done: EventHandler<()>) -> Element {
    let tx_backend = use_context::<mpsc::Sender<ToBackend>>();
    let ledger_nodes =
        use_context::<Signal<HashMap<String, crate::ledger_node::frontend::Frontend>>>();
    let mut errors = use_context::<Errors>();
    let i18n = use_i18n();
    let mut flow = use_signal(|| Flow::Choosing);
    let mut next_id = use_signal(|| 0u8);
    // Tracks whether the responder has submitted IDs (to hide input form and show progress).
    let mut submitted = use_signal(|| false);
    // Becomes true once a Pending entry has been observed; guards the established-dismiss check.
    let mut pending_was_seen = use_signal(|| false);

    // Find the relevant ConnectionEntry for the current flow.
    let active_entry: Option<ConnectionEntry> = match flow() {
        Flow::Choosing => None,
        Flow::Initiating { new_connection_id } | Flow::Responding { new_connection_id } => {
            ledger_nodes()
                .get(&private_key)
                .and_then(|f| {
                    f.connections_full
                        .iter()
                        .find(|e| e.new_connection_id() == Some(new_connection_id))
                })
                .cloned()
        }
    };

    // Once a Pending entry appears after submit, record it (false→true only, no loop).
    if submitted() && active_entry.is_some() && !pending_was_seen() {
        pending_was_seen.set(true);
    }

    // Auto-dismiss when a Failed entry is detected — push to error overlay then go back.
    if let Some(ConnectionEntry::Failed { .. }) = &active_entry {
        errors
            .write()
            .push(i18n.t(Key::ConnectionFailed).to_string());
        on_done.call(());
    }

    // Auto-dismiss when connection is established (Pending entry removed after being seen).
    if submitted() && pending_was_seen() && active_entry.is_none() {
        on_done.call(());
    }

    let back = move |_| {
        if flow() == Flow::Choosing {
            on_done.call(());
        } else {
            flow.set(Flow::Choosing);
        }
    };

    rsx! {
        div { class: "new-connection",
            div { class: "new-connection__header",
                button { class: "btn btn--secondary btn--sm", onclick: back, "{i18n.t(Key::Back)}" }
            }

            // Progress feedback — shown for any active Pending entry.
            if let Some(ConnectionEntry::Pending { progress: Some((status, pct)), .. }) = &active_entry {
                div { class: "new-connection__progress",
                    span { class: "new-connection__progress-label", "{status}" }
                    div { class: "new-connection__progress-track",
                        div {
                            class: "new-connection__progress-fill",
                            style: "width: {pct * 100.0:.0}%",
                        }
                    }
                }
            }

            match flow() {
                Flow::Choosing => rsx! {
                    div { class: "new-connection__choices",
                        button {
                            class: "btn btn--primary",
                            onclick: {
                                let tx = tx_backend.clone();
                                let private_key = private_key.clone();
                                move |_| {
                                    let id = next_id();
                                    next_id.set(id.wrapping_add(1));
                                    if let Ok(private) =
                                        blockchain::blockchain::Id::parse_str(&private_key)
                                    {
                                        send_or_error(
                                            &tx,
                                            ToBackend::Ledger {
                                                private,
                                                msg: LedgerToBackend::StartNewConnection {
                                                    new_connection_id: id,
                                                },
                                            },
                                            errors,
                                        );
                                    }
                                    flow.set(Flow::Initiating {
                                        new_connection_id: id,
                                    });
                                }
                            },
                            "{i18n.t(Key::InitiateConnection)}"
                        }
                        button {
                            class: "btn btn--secondary",
                            onclick: {
                                move |_| {
                                    let id = next_id();
                                    next_id.set(id.wrapping_add(1));
                                    flow.set(Flow::Responding {
                                        new_connection_id: id,
                                    });
                                }
                            },
                            "{i18n.t(Key::RespondToConnection)}"
                        }
                    }
                },
                Flow::Initiating { .. } => rsx! {
                    if let Some(ConnectionEntry::Pending { ids: Some(ids), .. }) = active_entry {
                        {
                            let invert_ids = crate::peer_connection::WebRtcIds {
                                room_id: ids.room_id.clone(),
                                own_id: ids.remote_id.clone(),
                                remote_id: ids.own_id.clone(),
                            };
                            rsx! {
                                if let Ok(json) = serde_json::to_string(&invert_ids) {
                                    BarcodeDisplay { json }
                                }
                                if cfg!(feature = "desktop") {
                                    WebRtcIdsShareView { ids: invert_ids }
                                }
                            }
                        }
                    } else {
                        p { class: "new-connection__waiting", "{i18n.t(Key::WaitingForIds)}" }
                    }
                },
                Flow::Responding { new_connection_id } => {
                    if submitted() {
                        rsx! {
                            p { class: "new-connection__waiting", "{i18n.t(Key::WaitingForConnection)}" }
                        }
                    } else {
                        let mut submit = {
                            let tx = tx_backend.clone();
                            let private_key = private_key.clone();
                            move |ids: WebRtcIds| {
                                if let Ok(private) = blockchain::blockchain::Id::parse_str(
                                    &private_key,
                                ) {
                                    send_or_error(
                                        &tx,
                                        ToBackend::Ledger {
                                            private,
                                            msg: LedgerToBackend::CreateNewConnectionFromWebRtcIds {
                                                new_connection_id,
                                                ids,
                                            },
                                        },
                                        errors,
                                    );
                                }
                                submitted.set(true);
                            }
                        };
                        rsx! {
                            BarcodeReader {
                                on_scan: {
                                    let mut submit = submit.clone();
                                    move |json: String| {
                                        if let Ok(ids) = serde_json::from_str::<WebRtcIds>(&json) {
                                            submit(ids);
                                        }
                                    }
                                },
                            }
                            if cfg!(feature = "desktop") {
                                WebRtcIdsInputView { on_submit: move |ids| submit(ids) }
                            }
                        }
                    }
                }
            }
        }
    }
}
