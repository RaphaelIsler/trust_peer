use dioxus::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{error, info};

#[cfg(any(feature = "desktop", feature = "mobile"))]
use p2p_webrtc::{P2pConfig, P2pWebRtc};

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq)]
enum ConnectionStatus {
    Idle,
    Connecting,
    Connected,
    Failed,
    Disconnected,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Connecting => write!(f, "Connecting..."),
            Self::Connected => write!(f, "Connected"),
            Self::Failed => write!(f, "Failed"),
            Self::Disconnected => write!(f, "Disconnected"),
        }
    }
}

/// P2P Test Component for testing WebRTC connections
#[component]
pub fn P2PTestComponent() -> Element {
    #[cfg(any(feature = "desktop", feature = "mobile"))]
    {
        let mut room_id = use_signal(|| String::new());
        let mut message_input = use_signal(|| String::new());
        let mut messages = use_signal(|| Vec::<String>::new());
        let mut connection_status = use_signal(|| ConnectionStatus::Idle);
        let mut error_message = use_signal(|| Option::<String>::None);
        let mut peer_id = use_signal(|| String::from("anonymous"));
        let mut signaling_server = use_signal(|| String::from("ws://127.0.0.1:3000"));

        // Shared P2P instance
        let mut p2p_instance: Signal<Option<Arc<Mutex<P2pWebRtc>>>> =
            use_signal(|| None);

        // Handle connection
        let handle_connect = move |_: dioxus::prelude::Event<dioxus::prelude::MouseData>| {
            spawn(async move {
                let room = room_id().trim().to_string();
                if room.is_empty() {
                    error_message.set(Some("Room ID cannot be empty".to_string()));
                    return;
                }

                connection_status.set(ConnectionStatus::Connecting);
                error_message.set(None);

                info!("[P2P] Attempting connection to room: {}", room);

                // Create config
                let config = P2pConfig::new(
                    signaling_server().clone(),
                    room.clone(),
                )
                .with_timeout(30)
                .with_peer_id(peer_id().clone());

                // Create P2P handler
                let mut p2p = P2pWebRtc::new(config);

                // Attempt connection
                match p2p.connect().await {
                    Ok(_) => {
                        info!("[P2P] Connected to room: {}", room);
                        connection_status.set(ConnectionStatus::Connected);
                        messages.write().push(format!(
                            "✓ Connected to room: {} (Peer: {})",
                            room,
                            peer_id()
                        ));

                        // Store P2P instance
                        let p2p_arc = Arc::new(Mutex::new(p2p));
                        p2p_instance.set(Some(p2p_arc.clone()));

                        // Spawn message receiver task
                        spawn(async move {
                            loop {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;

                                if let Some(arc) = p2p_instance() {
                                    let mut p2p_guard = arc.lock().await;
                                    match p2p_guard.try_recv().await {
                                        Ok(Some(data)) => {
                                            if let Ok(text) = String::from_utf8(data) {
                                                info!("[P2P] Received message: {}", text);
                                                messages.write().push(format!(
                                                    "← {}",
                                                    text
                                                ));
                                            }
                                        }
                                        Ok(None) => {
                                            // No message yet, continue polling
                                        }
                                        Err(e) => {
                                            error!("[P2P] Receive error: {}", e);
                                            break;
                                        }
                                    }
                                } else {
                                    break;
                                }
                            }
                        });
                    }
                    Err(e) => {
                        error!("[P2P] Connection failed: {}", e);
                        connection_status.set(ConnectionStatus::Failed);
                        error_message.set(Some(format!("Connection failed: {}", e)));
                        messages.write().push(format!("✗ Connection failed: {}", e));
                    }
                }
            });
        };

        // Handle send
        let handle_send = move |_: dioxus::prelude::Event<dioxus::prelude::MouseData>| {
            spawn(async move {
                let msg = message_input().trim().to_string();
                if msg.is_empty() {
                    return;
                }

                if let Some(p2p_arc) = p2p_instance() {
                    let mut p2p = p2p_arc.lock().await;
                    match p2p.send(msg.as_bytes()).await {
                        Ok(_) => {
                            info!("[P2P] Sent message: {}", msg);
                            messages.write().push(format!("→ {}", msg));
                            message_input.set(String::new());
                        }
                        Err(e) => {
                            error!("[P2P] Send error: {}", e);
                            error_message.set(Some(format!("Send failed: {}", e)));
                        }
                    }
                } else {
                    error_message.set(Some("Not connected".to_string()));
                }
            });
        };

        // Handle disconnect
        let handle_disconnect = move |_: dioxus::prelude::Event<dioxus::prelude::MouseData>| {
            spawn(async move {
                if let Some(p2p_arc) = p2p_instance() {
                    let mut p2p = p2p_arc.lock().await;
                    match p2p.close().await {
                        Ok(_) => {
                            info!("[P2P] Disconnected");
                            connection_status.set(ConnectionStatus::Disconnected);
                            messages.write().push("✓ Disconnected".to_string());
                            p2p_instance.set(None);
                        }
                        Err(e) => {
                            error!("[P2P] Disconnect error: {}", e);
                        }
                    }
                }
            });
        };

        rsx! {
            div {
                class: "p2p-test-component",
                style: "max-width: 800px; margin: 20px auto; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;",

                // Header
                div { style: "background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 20px; border-radius: 8px 8px 0 0;",
                    h2 { style: "margin: 0; font-size: 1.5em;", "🌐 P2P WebRTC Test" }
                    p { style: "margin: 10px 0 0 0; opacity: 0.9; font-size: 0.9em;",
                        "Test peer-to-peer connections with ICE candidates and DataChannel messaging"
                    }
                }

                // Configuration Section
                div { style: "background: #f8f9fa; padding: 20px; border-bottom: 1px solid #ddd;",

                    h3 { style: "margin-top: 0; font-size: 1.1em;", "Configuration" }

                    // Signaling Server
                    div { style: "margin-bottom: 15px;",
                        label { style: "display: block; margin-bottom: 5px; font-weight: 600; font-size: 0.9em;",
                            "Signaling Server"
                        }
                        input {
                            r#type: "text",
                            value: "{signaling_server}",
                            oninput: move |e| signaling_server.set(e.value()),
                            disabled: connection_status() != ConnectionStatus::Idle,
                            style: "width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 0.9em; box-sizing: border-box;",
                            placeholder: "ws://localhost:3000",
                        }
                    }

                    // Peer ID
                    div { style: "margin-bottom: 15px;",
                        label { style: "display: block; margin-bottom: 5px; font-weight: 600; font-size: 0.9em;",
                            "Peer ID"
                        }
                        input {
                            r#type: "text",
                            value: "{peer_id}",
                            oninput: move |e| peer_id.set(e.value()),
                            disabled: connection_status() != ConnectionStatus::Idle,
                            style: "width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 0.9em; box-sizing: border-box;",
                            placeholder: "anonymous",
                        }
                    }

                    // Room ID
                    div { style: "margin-bottom: 0;",
                        label { style: "display: block; margin-bottom: 5px; font-weight: 600; font-size: 0.9em;",
                            "Room ID *"
                        }
                        input {
                            r#type: "text",
                            value: "{room_id}",
                            oninput: move |e| room_id.set(e.value()),
                            disabled: connection_status() != ConnectionStatus::Idle,
                            style: "width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 0.9em; box-sizing: border-box;",
                            placeholder: "e.g. test-room-1",
                        }
                    }
                }

                // Connection Status
                div { style: "padding: 15px 20px; background: white; border-bottom: 1px solid #ddd;",

                    div { style: "display: flex; align-items: center; gap: 10px;",
                        div {
                            style: match connection_status() {
                                ConnectionStatus::Connected => {
                                    "width: 12px; height: 12px; border-radius: 50%; background: #28a745; animation: pulse 1s infinite;"
                                }
                                ConnectionStatus::Connecting => {
                                    "width: 12px; height: 12px; border-radius: 50%; background: #ffc107; animation: pulse 0.5s infinite;"
                                }
                                ConnectionStatus::Failed => {
                                    "width: 12px; height: 12px; border-radius: 50%; background: #dc3545;"
                                }
                                ConnectionStatus::Disconnected => {
                                    "width: 12px; height: 12px; border-radius: 50%; background: #6c757d;"
                                }
                                ConnectionStatus::Idle => {
                                    "width: 12px; height: 12px; border-radius: 50%; background: #ccc;"
                                }
                            },
                        }
                        span { style: "font-weight: 600; color: #333;", "Status: {connection_status}" }
                    }
                }

                // Error Message
                if let Some(err) = error_message() {
                    div { style: "background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; padding: 12px; margin: 15px 20px; color: #721c24; font-size: 0.9em;",
                        "⚠️ {err}"
                    }
                }

                // Control Buttons
                div { style: "padding: 15px 20px; display: flex; gap: 10px; border-bottom: 1px solid #ddd;",

                    button {
                        onclick: handle_connect,
                        disabled: connection_status() != ConnectionStatus::Idle,
                        style: match connection_status() {
                            ConnectionStatus::Idle => {
                                "flex: 1; padding: 10px 20px; background: #667eea; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600; transition: background 0.2s;"
                            }
                            _ => {
                                "flex: 1; padding: 10px 20px; background: #ccc; color: #666; border: none; border-radius: 4px; cursor: not-allowed; font-weight: 600;"
                            }
                        },
                        "🔗 Connect"
                    }

                    if connection_status() == ConnectionStatus::Connected
                        || connection_status() == ConnectionStatus::Connecting
                    {
                        button {
                            onclick: handle_disconnect,
                            style: "flex: 1; padding: 10px 20px; background: #dc3545; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600; transition: background 0.2s;",
                            "❌ Disconnect"
                        }
                    }
                }

                // Message Input
                if connection_status() == ConnectionStatus::Connected {
                    div { style: "padding: 15px 20px; background: #f8f9fa; border-bottom: 1px solid #ddd; display: flex; gap: 10px;",

                        input {
                            r#type: "text",
                            value: "{message_input}",
                            oninput: move |e| message_input.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == dioxus::prelude::Key::Enter {
                                    spawn(async move {
                                        let msg = message_input().trim().to_string();
                                        if msg.is_empty() {
                                            return;
                                        }

                                        if let Some(p2p_arc) = p2p_instance() {
                                            let mut p2p = p2p_arc.lock().await;
                                            match p2p.send(msg.as_bytes()).await {
                                                Ok(_) => {
                                                    info!("[P2P] Sent message: {}", msg);
                                                    messages.write().push(format!("→ {}", msg));
                                                    message_input.set(String::new());
                                                }
                                                Err(e) => {
                                                    error!("[P2P] Send error: {}", e);
                                                    error_message.set(Some(format!("Send failed: {}", e)));
                                                }
                                            }
                                        } else {
                                            error_message.set(Some("Not connected".to_string()));
                                        }
                                    });
                                }
                            },
                            style: "flex: 1; padding: 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 0.9em; font-family: 'Courier New', monospace;",
                            placeholder: "Enter message...",
                        }

                        button {
                            onclick: handle_send,
                            style: "padding: 10px 20px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600; white-space: nowrap;",
                            "📤 Send"
                        }
                    }
                }

                // Messages List
                div { style: "padding: 20px; background: white; min-height: 300px; max-height: 400px; overflow-y: auto; border-top: 2px solid #667eea;",

                    if messages.read().is_empty() {
                        div { style: "color: #999; font-style: italic; text-align: center; padding: 40px 20px;",
                            if connection_status() == ConnectionStatus::Idle {
                                "Enter a room ID and click Connect to start testing"
                            } else if connection_status() == ConnectionStatus::Connecting {
                                "Connecting... This may take a few seconds"
                            } else if connection_status() == ConnectionStatus::Failed {
                                "Connection failed. Check the server and try again"
                            } else {
                                "Messages will appear here. Send a message to test the connection"
                            }
                        }
                    } else {
                        div {
                            for (idx , msg) in messages.read().iter().enumerate() {
                                div {
                                    key: "{idx}",
                                    style: "padding: 8px 12px; margin: 5px 0; background: #f0f0f0; border-left: 3px solid #667eea; border-radius: 4px; font-family: 'Courier New', monospace; font-size: 0.85em; word-break: break-word;",
                                    "{msg}"
                                }
                            }
                        }
                    }
                }

                // Footer with info
                div { style: "padding: 15px 20px; background: #f8f9fa; border-top: 1px solid #ddd; color: #666; font-size: 0.85em; line-height: 1.6;",

                    p { style: "margin: 0;",
                        strong { "ℹ️ About: " }
                        "This component tests P2P WebRTC connections using the p2p_webrtc library. "
                        "Ensure a signaling server is running at the specified URL."
                    }

                    p { style: "margin: 8px 0 0 0;",
                        strong { "🔧 Status: " }
                        match connection_status() {
                            ConnectionStatus::Idle => "Ready to connect",
                            ConnectionStatus::Connecting => {
                                "Attempting connection, ICE gathering in progress..."
                            }
                            ConnectionStatus::Connected => "P2P connection established, DataChannel open",
                            ConnectionStatus::Failed => {
                                "Connection attempt failed, check console for details"
                            }
                            ConnectionStatus::Disconnected => "Connection closed, ready to reconnect",
                        }
                    }

                    p { style: "margin: 8px 0 0 0;",
                        strong { "📝 Debug: " }
                        "Check browser console (F12) for detailed logging of ICE candidates and events"
                    }
                }
            }
        }
    }

    #[cfg(not(any(feature = "desktop", feature = "mobile")))]
    {
        rsx! {
            div { style: "padding: 20px; background: #fff3cd; border: 1px solid #ffc107; border-radius: 4px; color: #856404;",
                p { "P2P WebRTC testing is only available for Desktop and Mobile platforms." }
            }
        }
    }
}
