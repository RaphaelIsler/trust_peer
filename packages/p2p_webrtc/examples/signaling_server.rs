//! Example: WebSocket Signaling Server
//!
//! A simple WebSocket signaling server for room-based P2P connections.
//! This server relays SDP offers/answers and ICE candidates between peers.
//!
//! Usage:
//!   cargo run --example signaling_server -- [port]
//!
//! Example:
//!   cargo run --example signaling_server -- 3000

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalingMessage {
    #[serde(rename = "join")]
    Join { room_id: String, peer_id: String },
    #[serde(rename = "offer")]
    Offer {
        from: String,
        to: String,
        sdp: String,
    },
    #[serde(rename = "answer")]
    Answer {
        from: String,
        to: String,
        sdp: String,
    },
    #[serde(rename = "ice")]
    IceCandidate {
        from: String,
        to: String,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u32,
    },
    #[serde(rename = "peer_joined")]
    PeerJoined { peer_id: String },
    #[serde(rename = "peer_left")]
    PeerLeft { peer_id: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

type PeerMap = Arc<RwLock<HashMap<String, mpsc::UnboundedSender<SignalingMessage>>>>;
type RoomMap = Arc<RwLock<HashMap<String, PeerMap>>>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).cloned().unwrap_or_else(|| "3000".to_string());
    let addr = format!("127.0.0.1:{}", port);

    let listener = TcpListener::bind(&addr).await?;
    info!("Signaling server listening on: ws://{}", addr);

    let rooms: RoomMap = Arc::new(RwLock::new(HashMap::new()));

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                info!("New connection from: {}", peer_addr);
                let rooms = Arc::clone(&rooms);
                tokio::spawn(handle_client(stream, rooms));
            }
            Err(e) => {
                error!("Accept error: {}", e);
            }
        }
    }
}

async fn handle_client(stream: TcpStream, rooms: RoomMap) {
    let addr = stream.local_addr().ok();

    match accept_async(stream).await {
        Ok(ws_stream) => {
            info!("WebSocket connection established");
            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            let (tx, mut rx) = mpsc::unbounded_channel();
            let mut room_id = String::new();
            let mut peer_id = String::new();

            // Message receive loop
            let mut receive_task = tokio::spawn(async {
                while let Some(Ok(msg)) = ws_receiver.next().await {
                    if let Ok(Message::Text(text)) = msg {
                        if let Ok(sig_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                            return (sig_msg, true);
                        }
                    }
                }
                (SignalingMessage::Ping, false)
            });

            // Message send loop
            let mut send_task = tokio::spawn(async {
                while let Some(msg) = rx.recv().await {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = ws_sender.send(Message::Text(json)).await;
                    }
                }
            });

            // Handle client messages
            loop {
                tokio::select! {
                    res = &mut receive_task => {
                        let (msg, ok) = match res {
                            Ok(v) => v,
                            Err(_) => break,
                        };

                        if !ok {
                            break;
                        }

                        match msg {
                            SignalingMessage::Join {
                                room_id: rid,
                                peer_id: pid,
                            } => {
                                room_id = rid.clone();
                                peer_id = pid.clone();

                                info!("Peer {} joined room {}", peer_id, room_id);

                                // Get or create room
                                let mut rooms_guard = rooms.write().await;
                                let peers = rooms_guard
                                    .entry(room_id.clone())
                                    .or_insert_with(|| Arc::new(RwLock::new(HashMap::new())));
                                drop(rooms_guard);

                                // Notify existing peers
                                let peers_guard = peers.read().await;
                                for (other_id, other_tx) in peers_guard.iter() {
                                    if other_id != &peer_id {
                                        let _ = other_tx.send(SignalingMessage::PeerJoined {
                                            peer_id: peer_id.clone(),
                                        });
                                    }
                                }
                                drop(peers_guard);

                                // Add this peer
                                let mut peers_guard = peers.write().await;
                                peers_guard.insert(peer_id.clone(), tx.clone());

                                // Notify this peer of existing peers
                                for other_id in peers_guard.keys() {
                                    if other_id != &peer_id {
                                        let _ = tx.send(SignalingMessage::PeerJoined {
                                            peer_id: other_id.clone(),
                                        });
                                    }
                                }
                            }
                            SignalingMessage::Offer { from, to, sdp } => {
                                info!("Offer from {} to {}", from, to);
                                forward_message(&rooms, &room_id, &to, msg).await;
                            }
                            SignalingMessage::Answer { from, to, sdp } => {
                                info!("Answer from {} to {}", from, to);
                                forward_message(&rooms, &room_id, &to, msg).await;
                            }
                            SignalingMessage::IceCandidate { from, to, .. } => {
                                debug_log(&format!("ICE candidate from {} to {}", from, to));
                                forward_message(&rooms, &room_id, &to, msg).await;
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Clean up when peer disconnects
            if !room_id.is_empty() && !peer_id.is_empty() {
                info!("Peer {} left room {}", peer_id, room_id);

                if let Some(peers) = rooms.read().await.get(&room_id) {
                    let mut peers_guard = peers.write().await;
                    peers_guard.remove(&peer_id);

                    for (_, other_tx) in peers_guard.iter() {
                        let _ = other_tx.send(SignalingMessage::PeerLeft {
                            peer_id: peer_id.clone(),
                        });
                    }
                }
            }
        }
        Err(e) => {
            error!("WebSocket error: {}", e);
        }
    }
}

async fn forward_message(
    rooms: &RoomMap,
    room_id: &str,
    to_peer: &str,
    msg: SignalingMessage,
) {
    if let Some(peers) = rooms.read().await.get(room_id) {
        if let Some(tx) = peers.read().await.get(to_peer) {
            let _ = tx.send(msg);
        }
    }
}

fn debug_log(msg: &str) {
    log::debug!("{}", msg);
}
