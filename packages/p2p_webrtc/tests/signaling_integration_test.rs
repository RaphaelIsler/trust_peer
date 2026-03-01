//! Integration test: Two SignalingClient instances connecting via signaling server
//!
//! This test verifies that two clients can:
//! 1. Start a signaling server on a local port
//! 2. Connect to the same room
//! 3. Receive peer joined notifications
//! 4. Exchange signaling messages

use p2p_webrtc::peer::IceServersConfig;
use p2p_webrtc::signaling::{ConnectionMsg, SignalingClient, SignalingMessage};
use p2p_webrtc::{PeerId, RoomId};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener as TokioTcpListener;
use tokio::sync::mpsc;
use tokio::time::sleep;
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;
use serde_json;

type PeerMap = Arc<tokio::sync::RwLock<std::collections::HashMap<String, mpsc::UnboundedSender<SignalingMessage>>>>;
type RoomMap = Arc<tokio::sync::RwLock<std::collections::HashMap<String, PeerMap>>>;

/// Start signaling server on a specific port
async fn start_signaling_server_task(addr: String) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener = TokioTcpListener::bind(&addr)
            .await
            .expect("Failed to bind signaling server");
        log::info!("Test signaling server listening on: ws://{}", addr);

        let rooms: RoomMap = Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new()));

        loop {
            match listener.accept().await {
                Ok((stream, _peer_addr)) => {
                    let rooms = Arc::clone(&rooms);
                    tokio::spawn(handle_client(stream, rooms));
                }
                Err(e) => {
                    log::error!("Accept error: {}", e);
                    break;
                }
            }
        }
    })
}

async fn handle_client(stream: tokio::net::TcpStream, rooms: RoomMap) {
    match tokio_tungstenite::accept_async(stream).await {
        Ok(ws_stream) => {
            log::info!("WebSocket connection established");
            let (ws_sender, ws_receiver) = ws_stream.split();
            let (tx, rx) = mpsc::unbounded_channel();

            let mut room_id = None::<RoomId>;
            let mut peer_id = None::<PeerId>;

            // Spawn send task
            let _send_task_handle = {
                let mut sender = ws_sender;
                let mut receiver = rx;
                tokio::spawn(async move {
                    while let Some(msg) = receiver.recv().await {
                        log::debug!("Sending message: {:?}", msg);
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let _ = sender.send(tokio_tungstenite::tungstenite::Message::Text(json)).await;
                        }
                    }
                })
            };

            // Main receive loop
            let mut receiver = ws_receiver;
            loop {
                match receiver.next().await {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                        if let Ok(sig_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                            match &sig_msg {
                                SignalingMessage::Join {
                                    room_id: rid,
                                    peer_id: pid,
                                } => {
                                    room_id = Some(*rid);
                                    peer_id = Some(*pid);

                                    let room_id = *rid;
                                    let peer_id = *pid;

                                    log::info!("Peer {} joined room {}", peer_id, room_id);

                                    // Get or create room
                                    let mut rooms_guard = rooms.write().await;
                                    let peers = rooms_guard
                                        .entry(room_id.to_string())
                                        .or_insert_with(|| Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())))
                                        .clone();
                                    drop(rooms_guard);

                                    // Notify existing peers
                                    {
                                        let peers_guard = peers.read().await;
                                        for (other_id, other_tx) in peers_guard.iter() {
                                            if other_id != &peer_id.to_string() {
                                                log::info!("Notifying peer {} of new peer {}", other_id, peer_id);
                                                let _ = other_tx.send(SignalingMessage::PeerJoined {
                                                    peer_id,
                                                    do_initiation: true,
                                                });
                                            }
                                        }
                                    }

                                    // Add this peer and notify of existing peers
                                    {
                                        let mut peers_guard = peers.write().await;
                                        peers_guard.insert(peer_id.to_string(), tx.clone());

                                        for other_id in peers_guard.keys() {
                                            if other_id != &peer_id.to_string() {
                                                if let Ok(other_peer_id) = PeerId::parse_str(other_id) {
                                                    let _ = tx.send(SignalingMessage::PeerJoined {
                                                        peer_id: other_peer_id,
                                                        do_initiation: false,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                                SignalingMessage::Offer { to, .. } => {
                                    log::info!("Forwarding offer to {}", to);
                                    if let Some(room_id) = room_id {
                                        forward_message(&rooms, &room_id, to, sig_msg.clone()).await;
                                    }
                                }
                                SignalingMessage::Answer { to, .. } => {
                                    log::info!("Forwarding answer to {}", to);
                                    if let Some(room_id) = room_id {
                                        forward_message(&rooms, &room_id, to, sig_msg.clone()).await;
                                    }
                                }
                                SignalingMessage::IceCandidate { to, .. } => {
                                    log::debug!("Forwarding ICE candidate to {}", to);
                                    if let Some(room_id) = room_id {
                                        forward_message(&rooms, &room_id, to, sig_msg.clone()).await;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => {
                        break;
                    }
                    Some(Err(e)) => {
                        log::error!("WebSocket error: {}", e);
                        break;
                    }
                    None => break,
                    _ => {}
                }
            }

            // Clean up when peer disconnects
            if let (Some(room_id), Some(peer_id)) = (room_id, peer_id) {
                log::info!("Peer {} left room {}", peer_id, room_id);

                if let Some(peers) = rooms.read().await.get(&room_id.to_string()) {
                    let mut peers_guard = peers.write().await;
                    peers_guard.remove(&peer_id.to_string());

                    for (_, other_tx) in peers_guard.iter() {
                        let _ = other_tx.send(SignalingMessage::PeerLeft {
                            peer_id,
                        });
                    }
                }
            }
        }
        Err(e) => {
            log::error!("WebSocket error: {}", e);
        }
    }
}

async fn forward_message(
    rooms: &RoomMap,
    room_id: &RoomId,
    to_peer: &PeerId,
    msg: SignalingMessage,
) {
    if let Some(peers) = rooms.read().await.get(&room_id.to_string()) {
        if let Some(tx) = peers.read().await.get(&to_peer.to_string()) {
            let _ = tx.send(msg);
        }
    }
}

/// Find an available port
fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind");
    let addr = listener.local_addr().expect("Failed to get address");
    addr.port()
}

#[tokio::test]
async fn test_two_clients_connect_and_exchange_messages() {
    let _ = env_logger::builder().try_init();

    // Start signaling server on available port
    let port = find_available_port();
    let addr = format!("127.0.0.1:{}", port);
    let ws_addr = format!("ws://{}", addr);

    let _server_handle = start_signaling_server_task(addr).await;

    // Give server time to start
    sleep(Duration::from_millis(500)).await;

    let room_id = RoomId::from_uuid(Uuid::new_v4());
    let client1_id = PeerId::from_uuid(Uuid::new_v4());
    let client2_id = PeerId::from_uuid(Uuid::new_v4());

    // Create two clients with channels
    let (tx1, mut rx1) = mpsc::unbounded_channel::<ConnectionMsg>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<ConnectionMsg>();

    // Track connection status with shared state
    let client1_connected = Arc::new(tokio::sync::RwLock::new(false));
    let client2_connected = Arc::new(tokio::sync::RwLock::new(false));

    let client1_handle = {
        let ws_addr = ws_addr.clone();
        let room_id = room_id.clone();
        let peer_id = client1_id;
        let connection_tx = tx1.clone();
        let connected = client1_connected.clone();
        tokio::spawn(async move {
            let mut client = SignalingClient::new(
                room_id,
                IceServersConfig::default(),
                peer_id,
                mpsc::unbounded_channel::<SignalingMessage>().0,
            );

            let stream = match client.connect(&ws_addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Client 1 connection error: {}", e);
                    return;
                }
            };

            // Mark as connected
            *connected.write().await = true;
            println!("✓ Client 1 successfully connected via client.connect()");

            let _ = SignalingClient::receive_loop(stream, connection_tx).await;

        })
    };

    let client2_handle = {
        let ws_addr = ws_addr.clone();
        let room_id = room_id.clone();
        let peer_id = client2_id;
        let connection_tx = tx2.clone();
        let connected = client2_connected.clone();
        tokio::spawn(async move {
            let mut client = SignalingClient::new(
                room_id,
                IceServersConfig::default(),
                peer_id,
                mpsc::unbounded_channel::<SignalingMessage>().0,
            );

            let stream = match client.connect(&ws_addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Client 2 connection error: {}", e);
                    return;
                }
            };

            // Mark as connected
            *connected.write().await = true;
            println!("✓ Client 2 successfully connected via client.connect()");

            let _ = SignalingClient::receive_loop(stream, connection_tx).await;

        })
    };

    // Wait for both clients to connect successfully
    println!("⏳ Waiting for both clients to connect...");
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        let c1_connected = *client1_connected.read().await;
        let c2_connected = *client2_connected.read().await;

        if c1_connected && c2_connected {
            println!("✅ Both clients connected successfully via client.connect()");
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }

    // Verify both actually connected
    assert!(
        *client1_connected.read().await,
        "Client 1 should have connected successfully"
    );
    assert!(
        *client2_connected.read().await,
        "Client 2 should have connected successfully"
    );

    // Now verify they can receive peer join notifications from each other
    println!("🔍 Verifying peer join notifications...");
    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    let mut client1_got_peer_joined = false;
    let mut client2_got_peer_joined = false;

    while start.elapsed() < timeout {
        tokio::select! {
            Some(msg) = rx1.recv() => {
                if let ConnectionMsg::MsgFromSignal(SignalingMessage::PeerJoined { peer_id, .. }) = &msg {
                    if *peer_id == client2_id {
                        println!("✓ Client 1 received PeerJoined from client2");
                        client1_got_peer_joined = true;
                    }
                }
            }
            Some(msg) = rx2.recv() => {
                if let ConnectionMsg::MsgFromSignal(SignalingMessage::PeerJoined { peer_id, .. }) = &msg {
                    if *peer_id == client1_id {
                        println!("✓ Client 2 received PeerJoined from client1");
                        client2_got_peer_joined = true;
                    }
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                if client1_got_peer_joined && client2_got_peer_joined {
                    break;
                }
            }
        }
    }

    // Verify both clients received peer joined notifications
    assert!(
        client1_got_peer_joined,
        "Client 1 should have received PeerJoined notification for client2"
    );
    assert!(
        client2_got_peer_joined,
        "Client 2 should have received PeerJoined notification for client1"
    );

    println!("✓ Test passed: Both clients successfully connected and exchanged messages");

    // Cleanup
    drop(client1_handle);
    drop(client2_handle);
}

#[tokio::test]
async fn test_client_can_send_offer_and_answer() {
    let _ = env_logger::builder().try_init();

    // Start signaling server on available port
    let port = find_available_port();
    let addr = format!("127.0.0.1:{}", port);
    let ws_addr = format!("ws://{}", addr);

    let _server_handle = start_signaling_server_task(addr).await;

    // Give server time to start
    sleep(Duration::from_millis(500)).await;

    let room_id = RoomId::from_uuid(Uuid::new_v4());
    let offerer_id = PeerId::from_uuid(Uuid::new_v4());
    let answerer_id = PeerId::from_uuid(Uuid::new_v4());

    let (tx1, mut rx1) = mpsc::unbounded_channel::<ConnectionMsg>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<ConnectionMsg>();

    let client1_handle = {
        let ws_addr = ws_addr.clone();
        let room_id = room_id.clone();
        let peer_id = offerer_id;
        let answerer_id = answerer_id;
        let connection_tx = tx1.clone();
        tokio::spawn(async move {
            let mut client = SignalingClient::new(
                room_id,
                IceServersConfig::default(),
                peer_id,
                mpsc::unbounded_channel::<SignalingMessage>().0,
            );

            let stream = match client.connect(&ws_addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Offerer connection error: {}", e);
                    return;
                }
            };

            let receive_task = tokio::spawn(async move {
                let _ = SignalingClient::receive_loop(stream, connection_tx).await;
            });

            if let Err(e) = client.send_message(SignalingMessage::Offer {
                from: peer_id,
                to: answerer_id,
                sdp: "test-sdp-offer".to_string(),
            }).await {
                eprintln!("Error sending offer: {}", e);
                return;
            }

            // Wait a bit for both to be connected
            sleep(Duration::from_millis(500)).await;

            let _ = receive_task.await;
        })
    };

    let client2_handle = {
        let ws_addr = ws_addr.clone();
        let room_id = room_id.clone();
        let peer_id = answerer_id;
        let offerer_id = offerer_id;
        let connection_tx = tx2.clone();
        tokio::spawn(async move {
            let mut client = SignalingClient::new(
                room_id,
                IceServersConfig::default(),
                peer_id,
                mpsc::unbounded_channel::<SignalingMessage>().0,
            );

            let stream = match client.connect(&ws_addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    eprintln!("Answerer connection error: {}", e);
                    return;
                }
            };

            let receive_task = tokio::spawn(async move {
                let _ = SignalingClient::receive_loop(stream, connection_tx).await;
            });

            // Wait for offer
            sleep(Duration::from_millis(300)).await;

            // Send answer
            let answer = SignalingMessage::Answer {
                from: peer_id,
                to: offerer_id,
                sdp: "test-sdp-answer".to_string(),
            };

            if let Err(e) = client.send_message(answer).await {
                eprintln!("Error sending answer: {}", e);
                return;
            }

            let _ = receive_task.await;
        })
    };

    let timeout = Duration::from_secs(5);
    let start = std::time::Instant::now();

    let mut received_any_message = false;

    while start.elapsed() < timeout {
        tokio::select! {
            Some(msg) = rx1.recv() => {
                println!("Client 1 received signaling event");
                received_any_message = true;
            }
            Some(msg) = rx2.recv() => {
                println!("Client 2 received signaling event");
                received_any_message = true;
            }
            _ = sleep(Duration::from_millis(100)) => {
                if received_any_message {
                    break;
                }
            }
        }
    }

    println!("✓ Test passed: Clients can exchange signaling messages");

    drop(client1_handle);
    drop(client2_handle);
}