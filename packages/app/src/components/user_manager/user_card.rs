use dioxus::prelude::*;

use crate::block_entry::{AppBlock, BlockView};
use crate::peer_connection::{
    Connection as PeerConnection, ConnectionView, WebRtcIds, WebRtcIdsInputView, WebRtcIdsShareView,
};
use crate::{LedgerNode, User};

#[component]
pub fn UserCard(user: User, service: LedgerNode) -> Element {
    let mut show_details = use_signal(|| false);
    let selected_chain = use_signal(|| None::<blockchain::blockchain::Id>);
    let mut private_id = use_signal(|| None::<blockchain::blockchain::Id>);
    let mut public_id = use_signal(|| None::<blockchain::blockchain::Id>);
    let active_chain = use_signal(|| None::<Vec<AppBlock>>);
    let mut chain_error = use_signal(|| None::<String>);
    let chain_loading = use_signal(|| false);
    let mut new_connection_ids = use_signal(|| None::<WebRtcIds>);
    let mut new_connection_error = use_signal(|| None::<String>);
    let mut imported_connection_message = use_signal(|| None::<String>);
    let mut peer_connections = use_signal(Vec::<PeerConnection>::new);
    let mut peer_connections_loading = use_signal(|| false);

    if private_id().is_none() {
        let service = service.clone();
        spawn(async move {
            match service.get_private_and_public().await {
                Ok((p_id, pub_id)) => {
                    private_id.set(Some(p_id));
                    public_id.set(Some(pub_id));
                }
                Err(e) => chain_error.set(Some(format!("Failed to load chain IDs: {e}"))),
            }
        });
    }

    if show_details() && !peer_connections_loading() && peer_connections().is_empty() {
        let service = service.clone();
        peer_connections_loading.set(true);
        spawn(async move {
            match service.get_peer_connections().await {
                Ok(connections) => {
                    peer_connections.set(connections);
                    chain_error.set(None);
                }
                Err(error) => {
                    chain_error.set(Some(format!("Failed to load peer connections: {error}")));
                }
            }
            peer_connections_loading.set(false);
        });
    }

    let chain_error_view = chain_error().map(|err| {
        rsx! {
            div { style: "margin: 10px 0; color: #b00020;", "{err}" }
        }
    });

    let chain_loading_view = if chain_loading() {
        Some(rsx! {
            div { style: "margin: 10px 0; color: #666;", "Loading blockchain..." }
        })
    } else {
        None
    };

    let chain_view = match active_chain() {
        Some(blocks) => Some(rsx! {
            div { class: "blockchain",
                if blocks.is_empty() {
                    div { class: "blockchain__empty", "No blocks" }
                }
                for (index, block) in blocks.into_iter().enumerate() {
                    BlockView { index, block }
                }
            }
        }),
        None => None,
    };

    rsx! {
        div {
            class: "user-card",
            style: "border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 8px; background: white;",

            div { style: "display: flex; justify-content: space-between; align-items: center;",
                div {
                    h4 { style: "margin: 0 0 5px 0;", "{user.full_name()}" }
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

                    div { style: "margin: 10px 0;",
                        strong { "ID: " }
                        span { style: "font-family: monospace; font-size: 0.9em;", "{user.id:?}" }
                    }

                    div { style: "margin: 10px 0;",
                        strong { "Created at:" }
                        span { style: "font-size: 0.9em;", "{user.created_at.to_rfc2822()}" }
                    }

                    div { style: "margin: 16px 0; display: flex; gap: 8px;",
                        button {
                            onclick: {
                                let active_chain = active_chain.clone();
                                let selected_chain = selected_chain.clone();
                                let chain_loading = chain_loading.clone();
                                let chain_error = chain_error.clone();
                                let service = service.clone();
                                move |_| select_chain(
                                    private_id(),
                                    active_chain,
                                    selected_chain,
                                    chain_loading,
                                    chain_error,
                                    service.clone(),
                                )
                            },
                            style: "padding: 6px 12px; background: #0d6efd; color: white; border: none; border-radius: 4px; cursor: pointer;",
                            "Private Chain"
                        }
                        button {
                            onclick: {
                                let active_chain = active_chain.clone();
                                let selected_chain = selected_chain.clone();
                                let chain_loading = chain_loading.clone();
                                let chain_error = chain_error.clone();
                                let service = service.clone();
                                move |_| select_chain(
                                    public_id(),
                                    active_chain,
                                    selected_chain,
                                    chain_loading,
                                    chain_error,
                                    service.clone(),
                                )
                            },
                            style: "padding: 6px 12px; background: #6610f2; color: white; border: none; border-radius: 4px; cursor: pointer;",
                            "Public Chain"
                        }
                        button {
                            onclick: {
                                let service = service.clone();
                                move |_| {
                                    let service = service.clone();
                                    spawn(async move {
                                        let (ids_tx, ids_rx) = tokio::sync::oneshot::channel();

                                        let service_done = service.clone();
                                        let mut new_connection_error_done = new_connection_error;
                                        let mut imported_connection_message_done =
                                            imported_connection_message;

                                        spawn(async move {
                                            match service_done.start_new_connection(ids_tx).await {
                                                Ok(()) => {
                                                    new_connection_error_done.set(None);
                                                    imported_connection_message_done
                                                        .set(Some("Verbindung aufgebaut".to_string()));
                                                }
                                                Err(error) => {
                                                    imported_connection_message_done.set(None);
                                                    new_connection_error_done.set(Some(format!(
                                                        "Verbindungsaufbau fehlgeschlagen: {error}"
                                                    )));
                                                }
                                            }
                                        });

                                        match ids_rx.await {
                                            Ok(Ok(ids)) => {
                                                imported_connection_message.set(Some(
                                                    "Verbindungsaufbau gestartet".to_string(),
                                                ));
                                                new_connection_ids.set(Some(ids));
                                            }
                                            Ok(Err(error)) => {
                                                imported_connection_message.set(None);
                                                new_connection_ids.set(None);
                                                new_connection_error.set(Some(format!(
                                                    "Failed to create connection: {error}"
                                                )));
                                            }
                                            Err(_) => {
                                                imported_connection_message.set(None);
                                                new_connection_ids.set(None);
                                                new_connection_error.set(Some(
                                                    "Failed to create connection".to_string(),
                                                ));
                                            }
                                        }
                                    });
                                }
                            },
                            style: "padding: 6px 12px; background: #198754; color: white; border: none; border-radius: 4px; cursor: pointer;",
                            "Neue Verbindung"
                        }
                    }

                    if let Some(error) = new_connection_error() {
                        div { style: "margin: 10px 0; color: #b00020;", "{error}" }
                    }

                    if let Some(ids) = new_connection_ids() {
                        div { style: "margin: 12px 0;",
                            WebRtcIdsShareView { ids }
                        }
                    }

                    div { style: "margin: 12px 0;",
                        WebRtcIdsInputView {
                            on_submit: {
                                let service = service.clone();
                                move |ids: WebRtcIds| {
                                    let service = service.clone();
                                    spawn(async move {
                                        match service
                                            .create_new_connection_from_web_rtc_ids(ids.clone())
                                            .await
                                        {
                                            Ok(()) => {
                                                new_connection_error.set(None);
                                                imported_connection_message
                                                    .set(Some("Verbindung gespeichert".to_string()));
                                                new_connection_ids.set(Some(ids));
                                            }
                                            Err(error) => {
                                                imported_connection_message.set(None);
                                                new_connection_error.set(Some(format!(
                                                    "Failed to save imported connection: {error}"
                                                )));
                                            }
                                        }
                                    });
                                }
                            },
                        }
                    }

                    if let Some(message) = imported_connection_message() {
                        div { style: "margin: 8px 0; color: #146c43;", "{message}" }
                    }

                    div { style: "margin: 16px 0;",
                        h5 { style: "margin: 0 0 8px 0;", "Peer Connections" }
                        if peer_connections_loading() {
                            div { style: "margin: 8px 0; color: #666;",
                                "Loading peer connections..."
                            }
                        } else if peer_connections().is_empty() {
                            div { style: "margin: 8px 0; color: #666; font-style: italic;",
                                "No peer connections"
                            }
                        } else {
                            for connection in peer_connections().iter() {
                                ConnectionView {
                                    connection: connection.clone(),
                                    on_save_name: {
                                        let service = service.clone();
                                        let mut peer_connections = peer_connections;
                                        let mut chain_error = chain_error;
                                        move |(id, name): (blockchain::blockchain::Id,
                                               Option<String>)| {
                                            let service = service.clone();
                                            let mut next_connections = peer_connections();
                                            spawn(async move {
                                                match service
                                                    .update_peer_connection_name(
                                                        id.clone(),
                                                        name.clone(),
                                                    )
                                                    .await
                                                {
                                                    Ok(()) => {
                                                        if let Some(connection) = next_connections
                                                            .iter_mut()
                                                            .find(|c| c.id == id)
                                                        {
                                                            connection.name = name;
                                                        }
                                                        peer_connections.set(next_connections);
                                                        chain_error.set(None);
                                                    }
                                                    Err(error) => {
                                                        chain_error.set(Some(format!(
                                                            "Failed to save peer connection name: \
                                                             {error}"
                                                        )));
                                                    }
                                                }
                                            });
                                        }
                                    },
                                }
                            }
                        }
                    }

                    {chain_error_view}
                    {chain_loading_view}
                    {chain_view}
                }
            }
        }
    }
}

fn select_chain(
    selection: Option<blockchain::blockchain::Id>,
    mut chain: Signal<Option<Vec<AppBlock>>>,
    mut selected_chain: Signal<Option<blockchain::blockchain::Id>>,
    mut chain_loading: Signal<bool>,
    mut chain_error: Signal<Option<String>>,
    service: crate::ledger_node::ui::LedgerNode,
) {
    if selection == selected_chain() {
        return;
    }
    if let Some(selection) = selection {
        chain_loading.set(true);
        chain_error.set(None);

        spawn(async move {
            chain.set(Some(
                service
                    .get_blocks(selection, 100, None)
                    .await
                    .unwrap_or_else(|e| {
                        chain_error.set(Some(format!("Failed to load blocks: {e}")));
                        Vec::new()
                    }),
            ));
            selected_chain.set(Some(selection));
            chain_loading.set(false);
        });
    }
}
