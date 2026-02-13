//! WebSocket signaling module
//!
//! Handles communication with the signaling server using WebSocket.
//! Manages room-based signaling for SDP and ICE candidate exchange.

use crate::error::{Error, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
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
    sink: Option<WsSink>,
    stream_rx: Option<WsStream2>,
    tx: mpsc::UnboundedSender<SignalingMessage>,
    rx: mpsc::UnboundedReceiver<SignalingMessage>,
    room_id: String,
    peer_id: String,
}

impl SignalingClient {
    /// Create a new signaling client (but don't connect yet)
    pub fn new(room_id: String, peer_id: String) -> (Self, mpsc::UnboundedReceiver<SignalingMessage>) {
        let (_tx, rx) = mpsc::unbounded_channel();
        let (_rx_out, rx_in) = mpsc::unbounded_channel();

        (
            Self {
                sink: None,
                stream_rx: None,
                tx: _rx_out,
                rx: rx_in,
                room_id,
                peer_id,
            },
            rx,
        )
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
        self.sink = Some(sink);
        self.stream_rx = Some(stream);

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
    pub async fn send_message(&mut self, msg: SignalingMessage) -> Result<()> {
        debug!("Sending signaling message: {:?}", msg);

        if let Some(sink) = &mut self.sink {
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
    pub async fn receive_loop(&mut self) -> Result<()> {
        if let Some(stream) = self.stream_rx.take() {
            let mut stream = stream;
            let tx = self.tx.clone();

            loop {
                match stream.next().await {
                    Some(Ok(Message::Text(text))) => {
                        debug!("Received signaling message: {}", text);
                        match serde_json::from_str::<SignalingMessage>(&text) {
                            Ok(msg) => {
                                if let Err(e) = tx.send(msg) {
                                    error!("Failed to forward signaling message: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse signaling message: {}", e);
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

    /// Get the next signaling message (non-blocking)
    pub fn try_recv(&mut self) -> Option<SignalingMessage> {
        self.rx.try_recv().ok()
    }
}
