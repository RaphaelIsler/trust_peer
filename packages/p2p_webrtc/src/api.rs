//! High-level async API for P2P WebRTC connections
//!
//! This module provides a simple, ergonomic API for establishing P2P connections,
//! sending/receiving data, and managing the connection lifecycle.

use crate::data_channel::DataChannel;
use crate::error::{Error, Result};
use crate::peer::{IceServersConfig, PeerConnection, TurnServer};
use crate::signaling::{SignalingClient, SignalingMessage};
use log::{debug, error, info};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use uuid::Uuid;

/// Configuration for P2P connection
#[derive(Debug, Clone)]
pub struct P2pConfig {
    /// WebSocket signaling server URL
    pub signaling_server: String,
    /// Room ID for the connection
    pub room_id: String,
    /// Optional peer ID (generated if not provided)
    pub peer_id: Option<String>,
    /// ICE servers configuration
    pub ice_config: IceServersConfig,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
}

impl P2pConfig {
    /// Create a new configuration with default values
    pub fn new(signaling_server: String, room_id: String) -> Self {
        Self {
            signaling_server,
            room_id,
            peer_id: None,
            ice_config: IceServersConfig::default(),
            connection_timeout: 30,
        }
    }

    /// Set custom STUN servers
    pub fn with_stun_servers(mut self, stun_servers: Vec<String>) -> Self {
        self.ice_config.stun_servers = stun_servers;
        self
    }

    /// Add a TURN server
    pub fn with_turn_server(
        mut self,
        urls: Vec<String>,
        username: String,
        credential: String,
    ) -> Self {
        self.ice_config.turn_servers.push(TurnServer {
            urls,
            username,
            credential,
        });
        self
    }

    /// Set peer ID
    pub fn with_peer_id(mut self, peer_id: String) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    /// Set connection timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.connection_timeout = seconds;
        self
    }
}

/// Main P2P WebRTC connection handler
pub struct P2pWebRtc {
    config: P2pConfig,
    peer_id: String,
    signaling: Arc<RwLock<SignalingClient>>,
    peer: Option<PeerConnection>,
    data_channel: Option<Arc<RwLock<DataChannel>>>,
    remote_peer_id: Arc<RwLock<Option<String>>>,
}

impl P2pWebRtc {
    /// Create a new P2P connection handler
    pub fn new(mut config: P2pConfig) -> Self {
        let peer_id = config
            .peer_id
            .take()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let (signaling, _) = SignalingClient::new(config.room_id.clone(), peer_id.clone());

        Self {
            config,
            peer_id,
            signaling: Arc::new(RwLock::new(signaling)),
            peer: None,
            data_channel: None,
            remote_peer_id: Arc::new(RwLock::new(None)),
        }
    }

    /// Connect to a peer in the room
    pub async fn connect(&mut self) -> Result<()> {
        info!("Starting P2P connection for room: {}", self.config.room_id);

        // Connect to signaling server
        self.signaling
            .write()
            .await
            .connect(&self.config.signaling_server)
            .await?;

        // Start receiving signaling messages
        let signaling = Arc::clone(&self.signaling);
        let remote_peer_id = Arc::clone(&self.remote_peer_id);
        let config = self.config.clone();
        let peer_id = self.peer_id.clone();
        let ice_config = self.config.ice_config.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::signaling_loop(
                signaling,
                remote_peer_id,
                config,
                peer_id,
                ice_config,
            )
            .await
            {
                error!("Signaling loop error: {}", e);
            }
        });

        // Wait for connection to be established
        let timeout = Duration::from_secs(self.config.connection_timeout);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(Error::Timeout);
            }

            // Check if data channel is ready
            if let Some(ref dc) = self.data_channel {
                if dc.read().await.is_open() {
                    info!("P2P connection established");
                    return Ok(());
                }
            }

            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Send data to the remote peer
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        if let Some(ref dc) = self.data_channel {
            dc.read().await.send(data).await
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Receive data from the remote peer (non-blocking)
    pub async fn try_recv(&self) -> Result<Option<Vec<u8>>> {
        if let Some(ref dc) = self.data_channel {
            Ok(dc.read().await.try_recv().await)
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Receive data from the remote peer (blocking)
    pub async fn recv(&self) -> Result<Vec<u8>> {
        if let Some(ref dc) = self.data_channel {
            dc.read()
                .await
                .recv()
                .await
                .ok_or(Error::NotConnected)
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        info!("Closing P2P connection");

        if let Some(ref dc) = self.data_channel {
            let _ = dc.read().await.close().await;
        }

        if let Some(ref peer) = self.peer {
            let _ = peer.close().await;
        }

        Ok(())
    }

    /// Internal signaling loop
    async fn signaling_loop(
        signaling: Arc<RwLock<SignalingClient>>,
        remote_peer_id: Arc<RwLock<Option<String>>>,
        _config: P2pConfig,
        peer_id: String,
        ice_config: IceServersConfig,
    ) -> Result<()> {
        let mut peer: Option<PeerConnection> = None;
        let mut _data_channel: Option<Arc<RwLock<DataChannel>>> = None;

        // Note: We can't easily run the receive loop in the background with the current design
        // This simplified version polls for messages

        // Main event loop
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;

            let mut signaling_mut = signaling.write().await;
            if let Some(msg) = signaling_mut.try_recv() {
                drop(signaling_mut);

                match msg {
                    SignalingMessage::PeerJoined { peer_id: remote_id } => {
                        if remote_id != peer_id {
                            info!("Peer joined: {}", remote_id);

                            // We are the initiator
                            *remote_peer_id.write().await = Some(remote_id.clone());

                            // Create peer connection
                            peer =
                                Some(PeerConnection::new(true, ice_config.clone()).await?);

                            // Create offer
                            let offer_sdp = peer.as_ref().unwrap().create_offer().await?;

                            // Send offer
                            let mut signaling_mut = signaling.write().await;
                            signaling_mut
                                .send_message(SignalingMessage::Offer {
                                    from: peer_id.clone(),
                                    to: remote_id.clone(),
                                    sdp: offer_sdp,
                                })
                                .await?;
                            drop(signaling_mut);
                        }
                    }
                    SignalingMessage::Offer {
                        from,
                        to,
                        sdp: offer_sdp,
                    } => {
                        if to == peer_id && from != peer_id {
                            info!("Received offer from: {}", from);

                            *remote_peer_id.write().await = Some(from.clone());

                            // Create peer connection
                            peer = Some(PeerConnection::new(false, ice_config.clone()).await?);

                            // Create answer
                            let answer_sdp =
                                peer.as_ref().unwrap().create_answer(&offer_sdp).await?;

                            // Send answer
                            let mut signaling_mut = signaling.write().await;
                            signaling_mut
                                .send_message(SignalingMessage::Answer {
                                    from: peer_id.clone(),
                                    to: from.clone(),
                                    sdp: answer_sdp,
                                })
                                .await?;
                            drop(signaling_mut);

                            // Create data channel (responder waits for incoming)
                            if let Some(ref p) = peer {
                                _data_channel = Some(Arc::new(RwLock::new(
                                    DataChannel::get_or_create(
                                        p.inner(),
                                        false, // responder
                                    )
                                    .await?,
                                )));
                            }
                        }
                    }
                    SignalingMessage::Answer {
                        from,
                        to,
                        sdp: answer_sdp,
                    } => {
                        if to == peer_id && from != peer_id {
                            info!("Received answer from: {}", from);

                            // Set remote answer
                            if let Some(ref p) = peer {
                                p.set_remote_answer(&answer_sdp).await?;

                                // Create data channel (initiator creates it)
                                _data_channel = Some(Arc::new(RwLock::new(
                                    DataChannel::get_or_create(p.inner(), true) // initiator
                                        .await?,
                                )));
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
                        if to == peer_id && from != peer_id {
                            debug!("Received ICE candidate from: {}", from);

                            if let Some(ref p) = peer {
                                let _ = p.add_ice_candidate(
                                    &candidate,
                                    &sdp_mid,
                                    sdp_mline_index,
                                )
                                .await;
                            }
                        }
                    }
                    _ => {
                        // Ignore other message types
                    }
                }
            }
        }
    }
}
