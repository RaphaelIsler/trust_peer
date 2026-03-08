//! DataChannel module
//!
//! Manages WebRTC DataChannels for reliable, ordered message delivery.

use crate::error::Error;
use anyhow::Result;

use crate::PeerId;
use log::{debug, info};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;

/// DataChannel wrapper for sending and receiving messages
#[derive(Clone)]
pub struct DataChannel {
    channel: Arc<RTCDataChannel>,
    pub own_peer_id: PeerId,
    pub remote_peer_id: PeerId,
}

impl DataChannel {
    /// Get or create a DataChannel named "data"
    pub async fn get_or_create(
        peer: Arc<RTCPeerConnection>,
        is_initiator: bool,
        own_peer_id: PeerId,
        remote_peer_id: PeerId,
    ) -> Result<Self> {
        if is_initiator {
            // Initiator creates the DataChannel
            debug!("Creating DataChannel 'data' (initiator)");

            let data_channel = peer
                .create_data_channel("data", None)
                .await
                .map_err(|e| Error::DataChannel(format!("Failed to create data channel: {}", e)))?;

            Self::setup_channel_handlers(Arc::clone(&data_channel)).await?;

            info!("DataChannel created successfully");
            Ok(Self {
                channel: data_channel,
                own_peer_id,
                remote_peer_id,
            })
        } else {
            // Responder waits for incoming DataChannel
            debug!("Waiting for DataChannel 'data' (responder)");

            let (channel_tx, mut channel_rx) = mpsc::unbounded_channel::<Arc<RTCDataChannel>>();
            let channel_tx = Arc::new(RwLock::new(Some(channel_tx)));

            // Set up on_data_channel callback
            let channel_tx_clone = Arc::clone(&channel_tx);
            peer.on_data_channel(Box::new(move |channel| {
                let tx = Arc::clone(&channel_tx_clone);
                Box::pin(async move {
                    if channel.label() == "data" {
                        debug!("Received DataChannel 'data'");
                        if let Ok(mut guard) = tx.try_write() {
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(channel);
                            }
                        }
                    }
                })
            }));

            // Wait for the channel to be received (with timeout)
            let data_channel =
                tokio::time::timeout(std::time::Duration::from_secs(10), channel_rx.recv())
                    .await
                    .map_err(|_| {
                        Error::DataChannel("Timeout waiting for data channel".to_string())
                    })?
                    .ok_or_else(|| Error::DataChannel("Data channel not received".to_string()))?;

            Self::setup_channel_handlers(Arc::clone(&data_channel)).await?;

            info!("DataChannel received successfully");
            Ok(Self {
                channel: data_channel,
                own_peer_id,
                remote_peer_id,
            })
        }
    }

    /// Setup handlers for data channel events
    async fn setup_channel_handlers(channel: Arc<RTCDataChannel>) -> Result<()> {
        // On message handler
        channel.on_message(Box::new(move |msg: DataChannelMessage| {
            debug!("Received data: {} bytes", msg.data.len());
            Box::pin(async {})
        }));

        // On error handler
        channel.on_error(Box::new(move |err| {
            log::error!("DataChannel error: {}", err);
            Box::pin(async {})
        }));

        // On close handler
        channel.on_close(Box::new(|| {
            info!("DataChannel closed");
            Box::pin(async {})
        }));

        // On open handler
        channel.on_open(Box::new(|| {
            info!("DataChannel opened");
            Box::pin(async {})
        }));

        Ok(())
    }

    /// Send data over the DataChannel
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        debug!("Sending {} bytes", data.len());

        let data_bytes = bytes::Bytes::copy_from_slice(data);

        self.channel
            .send(&data_bytes)
            .await
            .map_err(|e| Error::DataChannel(format!("Failed to send data: {}", e)))?;

        Ok(())
    }

    /// Receive data from the DataChannel (non-blocking)
    pub async fn try_recv(&self) -> Option<Vec<u8>> {
        // For now, this returns None as the current implementation
        // focuses on the signaling and connection setup
        // In a full implementation, you'd use message callbacks
        None
    }

    /// Receive data from the DataChannel (blocking)
    pub async fn recv(&self) -> Option<Vec<u8>> {
        // For now, this returns None as the current implementation
        // focuses on the signaling and connection setup
        None
    }

    /// Get the underlying WebRTC DataChannel
    pub fn inner(&self) -> Arc<RTCDataChannel> {
        Arc::clone(&self.channel)
    }

    /// Check if the channel is open
    pub fn is_open(&self) -> bool {
        self.channel.ready_state() as i32 == 1 // Open state
    }

    /// Close the DataChannel
    pub async fn close(&self) -> Result<()> {
        self.channel
            .close()
            .await
            .map_err(|e| Error::DataChannel(format!("Failed to close data channel: {}", e)))?;
        Ok(())
    }
}
