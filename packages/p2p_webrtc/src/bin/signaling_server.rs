//! WebSocket Signaling Server
//!
//! A simple WebSocket signaling server for room-based P2P connections.
//! This server relays SDP offers/answers and ICE candidates between peers.
//!
//! Usage:
//!   cargo run -p p2p_webrtc --bin signaling_server -- [port]
//!
//! Example:
//!   cargo run -p p2p_webrtc --bin signaling_server -- 3000

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use p2p_webrtc::signaling::SignalingMessage;


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
            Ok((stream, _peer_addr)) => {
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
    match accept_async(stream).await {
        Ok(ws_stream) => {
            info!("WebSocket connection established");
            let (ws_sender, ws_receiver) = ws_stream.split();
            let (tx, rx) = mpsc::unbounded_channel();

            let mut room_id = String::new();
            let mut peer_id = String::new();

            // Spawn send task
            let _send_task_handle = {
                let mut sender = ws_sender;
                let mut receiver = rx;
                tokio::spawn(async move {
                    while let Some(msg) = receiver.recv().await {
                        debug!("Sending message: {:?}", msg);

                        if let Ok(json) = serde_json::to_string(&msg) {
                            let _ = sender.send(Message::Text(json)).await;
                        }
                    }
                })
            };

            // Main receive loop
            let mut receiver = ws_receiver;
            loop {
                match receiver.next().await {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(sig_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                            match &sig_msg {
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
                                        .or_insert_with(|| Arc::new(RwLock::new(HashMap::new())))
                                        .clone();
                                    drop(rooms_guard);

                                    // Notify existing peers
                                    {
                                        let peers_guard = peers.read().await;
                                        for (other_id, other_tx) in peers_guard.iter() {
                                            if other_id != &peer_id {
                                                info!("Notifying peer {} of new peer {}", other_id, peer_id);
                                                let _ = other_tx.send(SignalingMessage::PeerJoined {
                                                    peer_id: peer_id.clone(),
                                                    do_initiation: true,
                                                });
                                            }
                                        }
                                    }

                                    // Add this peer and notify of existing peers
                                    {
                                        let mut peers_guard = peers.write().await;
                                        peers_guard.insert(peer_id.clone(), tx.clone());

                                        for other_id in peers_guard.keys() {
                                            if other_id != &peer_id {
                                                let _ = tx.send(SignalingMessage::PeerJoined {
                                                    peer_id: other_id.clone(),
                                                    do_initiation: false
                                                });
                                            }
                                        }
                                    }
                                }
                                SignalingMessage::Offer { to, .. } => {
                                    info!("Forwarding offer to {}", to);
                                    forward_message(&rooms, &room_id, to, sig_msg.clone()).await;
                                }
                                SignalingMessage::Answer { to, .. } => {
                                    info!("Forwarding answer to {}", to);
                                    forward_message(&rooms, &room_id, to, sig_msg.clone()).await;
                                }
                                SignalingMessage::IceCandidate { to, .. } => {
                                    debug!("Forwarding ICE candidate to {}", to);
                                    forward_message(&rooms, &room_id, to, sig_msg.clone()).await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
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
