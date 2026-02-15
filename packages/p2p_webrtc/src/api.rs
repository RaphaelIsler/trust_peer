//! High-level async API for P2P WebRTC connections
//!
//! This module provides a simple, ergonomic API for establishing P2P connections,
//! sending/receiving data, and managing the connection lifecycle.

use crate::data_channel::DataChannel;
use crate::error::{Error, Result};
use crate::peer::{IceServersConfig, PeerConnection, TurnServer};
use crate::signaling::{SignalingClient, SignalingMessage};
use log::info;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio::sync::mpsc;
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
    peer: Option<PeerConnection>,
    data_channel: Option<DataChannel>,
    #[allow(dead_code)]
    signaling_tx: mpsc::UnboundedSender<SignalingMessage>,
    signaling_rx: mpsc::UnboundedReceiver<SignalingMessage>,
    shutdown_tx: Option<mpsc::UnboundedSender<()>>,
}

impl P2pWebRtc {
    /// Create a new P2P connection handler
    pub fn new(mut config: P2pConfig) -> Self {
        let peer_id = config
            .peer_id
            .take()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        // Create the channel for signaling messages
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            config,
            peer_id,
            peer: None,
            data_channel: None,
            signaling_tx: tx,
            signaling_rx: rx,
            shutdown_tx: None,
        }
    }

    /// Connect to a peer in the room
    pub async fn connect(&mut self) -> Result<()> {
        info!("Starting P2P connection for room: {}", self.config.room_id);

        // Create signaling client (local to connect)
        let mut signaling = SignalingClient::new(
            self.config.room_id.clone(),
            self.config.ice_config.clone(),
            self.peer_id.clone(),
            self.signaling_tx.clone(),
        );

        // Establish connection and get data_channel arc and remote_peer_id
        let rx = {
            let (_, rx) = mpsc::unbounded_channel();
            std::mem::replace(&mut self.signaling_rx, rx)
        };
        self.data_channel = Some(signaling
            .establish_connection(
                &self.config.signaling_server,
                rx,
                Duration::from_secs(self.config.connection_timeout),
            )
            .await?);
        return Ok(());
/*
        self.data_channel = Some(data_channel.clone());
        self.shutdown_tx = Some(shutdown_tx);

        info!("Connected to remote peer: {}", remote_peer_id);

        // Wait for connection to be established
        let timeout = Duration::from_secs(self.config.connection_timeout);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                if let Some(tx) = &mut self.shutdown_tx {
                    let _ = tx.send(());
                }
                return Err(Error::Timeout);
            }

            // Check if data channel is ready
            if let Some(dc) = data_channel.read().await.as_ref() {
                if dc.is_open() {
                    info!("P2P connection established");
                    return Ok(());
                }
            }

            sleep(Duration::from_millis(100)).await;
        }*/
    }

    /// Send data to the remote peer
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        if let Some(dc) = &self.data_channel {
            dc.send(data).await
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Receive data from the remote peer (non-blocking)
    pub async fn try_recv(&self) -> Result<Option<Vec<u8>>> {
        if let Some(dc) = &self.data_channel {
            Ok(dc.try_recv().await)
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Receive data from the remote peer (blocking)
    pub async fn recv(&self) -> Result<Vec<u8>> {
        if let Some(dc) = &self.data_channel {
            dc.recv()
                .await
                .ok_or(Error::NotConnected)
        } else {
            Err(Error::NotConnected)
        }
    }

    /// Close the connection
    pub async fn close(&mut self) -> Result<()> {
        info!("Closing P2P connection");

        if let Some(dc) = &self.data_channel {
            let _ = dc.close().await;
        }

        if let Some(peer) = &self.peer {
            let _ = peer.close().await;
        }

        Ok(())
    }
}
