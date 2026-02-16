//! WebSocket signaling module
//!
//! Handles communication with the signaling server using WebSocket.
//! Manages room-based signaling for SDP and ICE candidate exchange.

use crate::error::{Error, Result};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use super::data_channel::DataChannel;
use super::peer::PeerConnection;
use core::time;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
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
        do_initiation: bool,
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


#[derive(Clone)]
pub enum ConnectionMsg{
    Timeout,
//    DataChannelReady(DataChannel),
    MsgFromSignal(SignalingMessage),
    SendIceCandidate(SignalingMessage)
}

/// Signaling client for WebSocket connection to signaling server
pub struct SignalingClient {
    sink: Arc<tokio::sync::Mutex<Option<WsSink>>>,
  //  stream_rx: Arc<tokio::sync::Mutex<Option<WsStream2>>>,
//    tx: mpsc::UnboundedSender<SignalingMessage>,
    ice_config: crate::peer::IceServersConfig,
    room_id: String,
    peer_id: String,
    remote_id: Option<String>,
    peer: Option<crate::peer::PeerConnection>,
    ice_rx: Option<mpsc::UnboundedReceiver<Option<crate::peer::IceCandidate>>>,
    data_channel: Option<DataChannel>,
}


impl SignalingClient {
    /// Create a new signaling client (but don't connect yet)
    pub fn new(room_id: String,  ice_config: crate::peer::IceServersConfig, peer_id: String, tx: mpsc::UnboundedSender<SignalingMessage>) -> Self {
        Self {
            sink: Arc::new(tokio::sync::Mutex::new(None)),
            //stream_rx: Arc::new(tokio::sync::Mutex::new(None)),
//            tx,
            ice_config,
            room_id,
            peer_id,
            remote_id: None,
            peer: None,
            ice_rx: None,
            data_channel: None,
        }
    }

    /// Connect to the signaling server
    pub async fn connect(&mut self, signaling_server: &str) -> Result<WsStream2> {
        info!(
            "Connecting to signaling server: {} (room: {}, peer: {})",
            signaling_server, self.room_id, self.peer_id
        );

        let (ws_stream, _) = connect_async(signaling_server)
            .await
            .map_err(|e| Error::Signaling(format!("Failed to connect to signaling server: {}", e)))?;

        let (sink, stream) = ws_stream.split();
        *self.sink.lock().await = Some(sink);
//        *self.stream_rx.lock().await = Some(stream);

        info!("Connected to signaling server");

        // Send join message
        self.send_message(SignalingMessage::Join {
            room_id: self.room_id.clone(),
            peer_id: self.peer_id.clone(),
        })
        .await?;

        Ok(stream)
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
    pub async fn receive_loop(mut stream: WsStream2, tx: mpsc::UnboundedSender<ConnectionMsg>) -> Result<()> {
        info!("receive_loop started");
        loop {
            match stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    debug!("Received raw WebSocket text: {}", text);
                    match serde_json::from_str::<SignalingMessage>(&text) {
                        Ok(msg) => {
                            info!("Parsed signaling message: {:?}", msg);
                            if let Err(e) = tx.send(ConnectionMsg::MsgFromSignal(msg)) {
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
        Ok(())
    }


    /// Establish a complete P2P connection with all setup
    pub async fn establish_connection(
        mut self,
        signaling_server: &str,
      //  ice_config: crate::peer::IceServersConfig,
//        rx: mpsc::UnboundedReceiver<SignalingMessage>,
        connection_timeout: time::Duration
    ) -> Result<DataChannel>
    {
        info!("Establishing P2P connection via signaling");

        // Connect to signaling server
        let stream = self.connect(signaling_server).await?;

        let (connection_tx, mut connection_rx) = mpsc::unbounded_channel();
        { let connection_tx = connection_tx.clone();
            // Start receiving signaling messages in background
            tokio::spawn(async move {
                info!("Starting signaling receive loop");
                if let Err(e) = Self::receive_loop(stream, connection_tx.clone()).await {
                    error!("Signaling receive loop error: {}", e);
                }
            });
        }


        // Wait for remote peer ID to be set
        {
            let connection_tx = connection_tx.clone();
            tokio::spawn(async move { Self::timeout_task(connection_tx, connection_timeout) });
        }


        let (dc_tx, dc_rx) = oneshot::channel();

        // Loop im Hintergrund laufen lassen, damit ICE weiter verarbeitet wird
        let mut client = self;
        tokio::spawn(async move {
            if let Err(e) = client.run_connection_loop(connection_rx, dc_tx).await {
                error!("Connection loop error: {}", e);
            }
        });
        match dc_rx.await {
            Ok(dc) => Ok(dc),
            Err(_) => Err(Error::Timeout),
        }
    }

    async fn run_connection_loop(
        &mut self,
        mut connection_rx: mpsc::UnboundedReceiver<ConnectionMsg>,
        mut dc_tx: oneshot::Sender<DataChannel>,
    ) -> Result<()> {
        let mut dc_tx = Some(dc_tx);
        loop {
            tokio::select! {
                // Listen for connection messages
                Some(msg) = connection_rx.recv() => {
                    match msg{
                        ConnectionMsg::Timeout => {
                            info!("Received shutdown message, exiting");
                            return Err(Error::Timeout);
                        }
                       /*  ConnectionMsg::DataChannelReady(dc) => {
                            if !dc_sent {
                                info!("Received DataChannel ready message, returning connection");
                                let _ = dc_tx.send(dc);
                                dc_sent = true;
                            }
                            //return Ok(dc);
                        }*/
                        ConnectionMsg::MsgFromSignal(msg) => {
                            match self.handle_msg(msg).await {
                                Ok(Some(dc)) => {
                                    info!("DataChannel is ready, returning connection");
                                    if let Some(tx) = dc_tx.take() {
                                        let _ = tx.send(dc);
                                    }
                                }
                                Ok(None) => {
                                    debug!("Message handled but DataChannel not ready yet");
                                }
                                Err(e) => {
                                    error!("Error handling signaling message: {}", e);
                                }
                            }
                        }
                        ConnectionMsg::SendIceCandidate(msg) => {
                            if let Err(e) = self.send_message(msg).await {
                                error!("Failed to send ICE candidate message: {}", e);
                            }
                        }
                    }
                }
                else => break,
            }
        }
        Ok(())
    }

    fn timeout_task(rx: mpsc::UnboundedSender<ConnectionMsg>, timeout: time::Duration) {
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            let _ = rx.send(ConnectionMsg::Timeout);
        });
    }


    /// Create a clone of the signaling client with shared internal state
    /*fn clone_inner(&self) -> Self {
        Self {
            sink: Arc::clone(&self.sink),
//            stream_rx: Arc::clone(&self.stream_rx),
            tx: self.tx.clone(),
            room_id: self.room_id.clone(),
            peer_id: self.peer_id.clone(),
            remote_id: self.remote_id.clone()
        }
    }*/

    pub async fn handle_msg(&mut self, msg: SignalingMessage) -> Result<Option<DataChannel>> {
        match msg {
            SignalingMessage::PeerJoined { peer_id: remote_id, do_initiation: is_initiator } => {
                if remote_id != self.peer_id {
                    self.remote_id = Some(remote_id.clone());
                    info!("Peer joined: {} is_initiator: {}", remote_id.clone(), is_initiator);

                    let (peer, ice_rx) = PeerConnection::new(is_initiator, self.ice_config.clone()).await?;
                    self.peer = Some(peer);
                    self.ice_rx = Some(ice_rx);

                    if is_initiator {
                        info!("Initiator: Creating and sending offer...");

                        // Spawn task to gather and send ICE candidates
                        self.data_channel = Some(crate::data_channel::DataChannel::get_or_create(
                            self.peer.as_ref().unwrap().inner(),
                            true,
                            self.peer_id.clone(),
                            remote_id.clone(),
                        ).await?
                        );
                        let offer_sdp = self.peer.as_ref().unwrap().create_offer().await?;
                        info!("Offer SDP created, length: {}", offer_sdp.len());

                        self.gather_and_send_ice_candidates().await?;


                        info!("Sending offer from {} to {}", self.peer_id, remote_id);
                        self.send_message(SignalingMessage::Offer {
                                from: self.peer_id.clone(),
                                to: remote_id.clone(),
                                sdp: offer_sdp,
                            })
                            .await
                            .map_err(|e| {
                                error!("Failed to send offer: {}", e);
                                e
                            })?;
                        info!("Offer sent successfully!");

                    } else {
                        info!("Not initiator, waiting for offer from: {}", remote_id);
                    }
                }
            }
            SignalingMessage::Offer {
                from,
                to,
                sdp: offer_sdp,
            } => {
                info!("Offer message received: from={}, to={}, expected_to={}", from, to, self.peer_id);
                if to == self.peer_id && from != self.peer_id {
                    info!("Received offer from: {}", from);
                    self.remote_id = Some(from.clone());

                    if self.peer.is_none() {
                        let (peer, ice_rx) = PeerConnection::new(false, self.ice_config.clone()).await?;
                        self.peer = Some(peer);
                        self.ice_rx = Some(ice_rx);
                    }

                    info!("Creating answer...");
                    let answer_sdp = self.peer.as_ref().unwrap().create_answer(&offer_sdp).await?;
                    info!("Answer created, length: {}", answer_sdp.len());

                                        // Spawn task to gather and send ICE candidates
//                    if let Some(ice_rx) = self.ice_rx.take() {
         //           self.gather_and_send_ice_candidates().await?;
  //                  }

//                    self.gather_and_send_ice_candidates().await?;

                    info!("Sending answer from {} to {}", self.peer_id, from);
                    self.send_message(SignalingMessage::Answer {
                            from: self.peer_id.clone(),
                            to: from.clone(),
                            sdp: answer_sdp,
                        })
                        .await
                        .map_err(|e| {
                            error!("Failed to send answer: {}", e);
                            e
                        })?;
                    info!("Answer sent successfully!");

                    self.gather_and_send_ice_candidates().await?;

                    if let Some(ref p) = self.peer {
                        info!("Creating data channel for responder...");
                        let dc = crate::data_channel::DataChannel::get_or_create(
                            p.inner(),
                            false,
                            self.peer_id.clone(),
                            from.clone(),
                        )
                        .await?;
                        info!("Data channel created for responder");
                        return Ok(Some(dc));
                    }
                }
            }
            SignalingMessage::Answer {
                from,
                to,
                sdp: answer_sdp,
            } => {
                if to == self.peer_id && from != self.peer_id {
                    info!("Received answer from: {}", from);
                    if let Some(ref p) = self.peer {
                        p.set_remote_answer(&answer_sdp).await?;


                        if let Some(dc) = self.data_channel.take() {
                            return Ok(Some(dc));
                        }

                        //
                        let dc = crate::data_channel::DataChannel::get_or_create(
                            p.inner(),
                            true,
                            self.peer_id.clone(),
                            from.clone(),
                        )
                        .await?;
                        info!("Data channel created for responder");
                        return Ok(Some(dc));
                    }
                }
            }
            SignalingMessage::IceCandidate {
                from,
                to,
                candidate,
                sdp_mid,
                sdp_mline_index,
            } => {
                if to == self.peer_id && from != self.peer_id {
                    debug!("Received ICE candidate from: {}", from);
                    if let Some(ref p) = self.peer {
                        let _ = p.add_ice_candidate(&candidate, &sdp_mid, sdp_mline_index)
                            .await;
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    /// Helper function to gather ICE candidates and send them via signaling
    async fn gather_and_send_ice_candidates(
        &mut self,
//        mut ice_rx: mpsc::UnboundedReceiver<Option<IecCandidate>>,
  //      peer_id: String,
    //    remote_id: String,
//        tx: mpsc::UnboundedSender<SignalingMessage>,
    ) -> Result<()> {
        let mut ice_rx = self.ice_rx.take();
        if let (Some(remote_id), Some(ice_rx)) = (&self.remote_id, &mut ice_rx){
            while let Some(Some(candidate)) = ice_rx.recv().await {
                info!(
                    "Sending ICE candidate from {} to {}: {}",
                    self.peer_id, remote_id, candidate.candidate
                );
                let msg = SignalingMessage::IceCandidate {
                    from: self.peer_id.clone(),
                    to: remote_id.clone(),
                    candidate: candidate.candidate,
                    sdp_mid: candidate.sdp_mid,
                    sdp_mline_index: candidate.sdp_mline_index,
                };
                self.send_message(msg).await
                    .map_err(|e| {
                            error!("Failed to send answer: {}", e);
                            e
                        })?;

    //            let _ = tx.send(msg);
            }
            debug!("ICE candidate gathering task finished for {}", self.peer_id);
        } else {
            warn!("ICE candidate gathering task: no remote_id or ice_rx set");

        }
        self.ice_rx = ice_rx;
        Ok(())
    }

    // Internal signaling loop - handles all signaling messages and peer connection setup
}
