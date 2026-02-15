//! WebSocket signaling module
//!
//! Handles communication with the signaling server using WebSocket.
//! Manages room-based signaling for SDP and ICE candidate exchange.

use crate::error::{Error, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, WebSocketStream};

type WsStream = WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsStream2 = SplitStream<WsStream>;

/// Messages exchanged via signaling server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalingMessage {
    /// Join a room
    #[serde(rename = "join")]
    Join {
        room_id: String,
        peer_id: String,
    },
    /// SDP offer
    #[serde(rename = "offer")]
    Offer {
        from: String,
        to: String,
        sdp: String,
    },
    /// SDP answer
    #[serde(rename = "answer")]
    Answer {
        from: String,
        to: String,
        sdp: String,
    },
    /// ICE candidate
    #[serde(rename = "ice")]
    IceCandidate {
        from: String,
        to: String,
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u32,
    },
    /// Notification of peer in room
    #[serde(rename = "peer_joined")]
    PeerJoined {
        peer_id: String,
    },
    /// Notification of peer leaving room
    #[serde(rename = "peer_left")]
    PeerLeft {
        peer_id: String,
    },
    /// Keep-alive ping
    #[serde(rename = "ping")]
    Ping,
    /// Keep-alive pong
    #[serde(rename = "pong")]
    Pong,
}

/// Signaling client for WebSocket connection to signaling server
pub struct SignalingClient {
    sink: Arc<tokio::sync::Mutex<Option<WsSink>>>,
    stream_rx: Arc<tokio::sync::Mutex<Option<WsStream2>>>,
    tx: mpsc::UnboundedSender<SignalingMessage>,
    room_id: String,
    peer_id: String,
}

impl SignalingClient {
    /// Create a new signaling client (but don't connect yet)
    pub fn new(room_id: String, peer_id: String, tx: mpsc::UnboundedSender<SignalingMessage>) -> Self {
        Self {
            sink: Arc::new(tokio::sync::Mutex::new(None)),
            stream_rx: Arc::new(tokio::sync::Mutex::new(None)),
            tx,
            room_id,
            peer_id,
        }
    }

    /// Connect to the signaling server
    pub async fn connect(&mut self, signaling_server: &str) -> Result<()> {
        info!(
            "Connecting to signaling server: {} (room: {}, peer: {})",
            signaling_server, self.room_id, self.peer_id
        );

        let (ws_stream, _) = connect_async(signaling_server)
            .await
            .map_err(|e| Error::Signaling(format!("Failed to connect to signaling server: {}", e)))?;

        let (sink, stream) = ws_stream.split();
        *self.sink.lock().await = Some(sink);
        *self.stream_rx.lock().await = Some(stream);

        info!("Connected to signaling server");

        // Send join message
        self.send_message(SignalingMessage::Join {
            room_id: self.room_id.clone(),
            peer_id: self.peer_id.clone(),
        })
        .await?;

        Ok(())
    }

    /// Send a signaling message
    pub async fn send_message(&self, msg: SignalingMessage) -> Result<()> {
        debug!("Sending signaling message: {:?}", msg);

        if let Some(sink) = &mut *self.sink.lock().await {
            let json = serde_json::to_string(&msg)?;
            sink.send(Message::Text(json))
                .await
                .map_err(|e| Error::Signaling(format!("Failed to send message: {}", e)))?;
        } else {
            return Err(Error::Signaling(
                "Signaling client not connected".to_string(),
            ));
        }

        Ok(())
    }

    /// Receive signaling messages in a loop (should be spawned as a task)
    pub async fn receive_loop(&self) -> Result<()> {
        info!("receive_loop started");
        if let Some(stream) = self.stream_rx.lock().await.take() {
            let mut stream = stream;
            let tx = self.tx.clone();

            loop {
                match stream.next().await {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received raw WebSocket text: {}", text);
                        match serde_json::from_str::<SignalingMessage>(&text) {
                            Ok(msg) => {
                                info!("Parsed signaling message: {:?}", msg);
                                if let Err(e) = tx.send(msg) {
                                    error!("Failed to forward signaling message to channel: {}", e);
                                    break;
                                } else {
                                    debug!("Successfully forwarded message to channel");
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse signaling message: {} - text was: {}", e, text);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Signaling server closed connection");
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("Signaling stream ended");
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}
