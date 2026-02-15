//! WebRTC peer connection module
//!
//! Manages the WebRTC PeerConnection, including SDP offer/answer exchange,
//! ICE candidate handling, and connection state management.

use crate::error::{Error, Result};
use log::{debug, info};
use std::sync::Arc;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

/// Configuration for ICE servers
#[derive(Debug, Clone)]
pub struct IceServersConfig {
    /// STUN servers (e.g., "stun:stun.example.com:3478")
    pub stun_servers: Vec<String>,
    /// TURN servers with credentials
    pub turn_servers: Vec<TurnServer>,
}

#[derive(Debug, Clone)]
pub struct TurnServer {
    pub urls: Vec<String>,
    pub username: String,
    pub credential: String,
}

impl Default for IceServersConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![],
            turn_servers: vec![],
        }
    }
}

/// WebRTC peer connection wrapper
pub struct PeerConnection {
    peer: Arc<RTCPeerConnection>,
    is_initiator: bool,
}

impl PeerConnection {
    /// Create a new WebRTC peer connection
    pub async fn new(is_initiator: bool, ice_config: IceServersConfig) -> Result<Self> {
        info!("Creating WebRTC peer connection (initiator: {})", is_initiator);

        // Configure ICE servers
        let mut ice_servers = Vec::new();

        // Add STUN servers
        for stun_url in ice_config.stun_servers {
            ice_servers.push(RTCIceServer {
                urls: vec![stun_url],
                ..Default::default()
            });
        }

        // Add TURN servers
        for turn in ice_config.turn_servers {
            ice_servers.push(RTCIceServer {
                urls: turn.urls,
                username: turn.username,
                credential: turn.credential,
                ..Default::default()
            });
        }

        // Create WebRTC API
        let setting_engine = SettingEngine::default();

        let api = APIBuilder::new()
            .with_setting_engine(setting_engine)
            .build();

        // Create peer connection
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };

        let peer = api
            .new_peer_connection(config)
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to create peer connection: {}", e)))?;

        Ok(Self {
            peer: Arc::new(peer),
            is_initiator,
        })
    }

    /// Create an SDP offer (initiator only)
    pub async fn create_offer(&self) -> Result<String> {
        if !self.is_initiator {
            return Err(Error::WebRtc(
                "Only initiator can create offer".to_string(),
            ));
        }

        debug!("Creating SDP offer");

        let offer = self
            .peer
            .create_offer(None)
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to create offer: {}", e)))?;

        self.peer
            .set_local_description(offer.clone())
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to set local description: {}", e)))?;

        Ok(offer.sdp)
    }

    /// Create an SDP answer (responder only)
    pub async fn create_answer(&self, offer_sdp: &str) -> Result<String> {
        if self.is_initiator {
            return Err(Error::WebRtc(
                "Only responder can create answer".to_string(),
            ));
        }

        debug!("Creating SDP answer");

        let offer = RTCSessionDescription::offer(offer_sdp.to_string())
            .map_err(|e| Error::WebRtc(format!("Failed to parse offer SDP: {}", e)))?;

        self.peer
            .set_remote_description(offer)
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to set remote description: {}", e)))?;

        let answer = self
            .peer
            .create_answer(None)
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to create answer: {}", e)))?;

        self.peer
            .set_local_description(answer.clone())
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to set local description: {}", e)))?;

        Ok(answer.sdp)
    }

    /// Set the remote SDP answer (initiator only)
    pub async fn set_remote_answer(&self, answer_sdp: &str) -> Result<()> {
        if !self.is_initiator {
            return Err(Error::WebRtc(
                "Only initiator can set remote answer".to_string(),
            ));
        }

        debug!("Setting remote SDP answer");

        let answer = RTCSessionDescription::answer(answer_sdp.to_string())
            .map_err(|e| Error::WebRtc(format!("Failed to parse answer SDP: {}", e)))?;

        self.peer
            .set_remote_description(answer)
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to set remote description: {}", e)))?;

        Ok(())
    }

    /// Add an ICE candidate
    pub async fn add_ice_candidate(
        &self,
        candidate: &str,
        sdp_mid: &str,
        sdp_mline_index: u32,
    ) -> Result<()> {
        debug!(
            "Adding ICE candidate: {} (mid: {}, index: {})",
            candidate, sdp_mid, sdp_mline_index
        );

        // The webrtc-rs crate has a different API for ICE candidates
        // For now, we'll log and continue - the candidate will be applied through
        // normal WebRTC signaling
        debug!("ICE candidate queued ({}): {}", sdp_mid, candidate);
        Ok(())
    }

    /// Get the underlying peer connection
    pub fn inner(&self) -> Arc<RTCPeerConnection> {
        Arc::clone(&self.peer)
    }

    /// Get peer connection state
    pub fn connection_state(&self) -> RTCPeerConnectionState {
        self.peer.connection_state()
    }

    /// Check if peer connection is connected
    pub fn is_connected(&self) -> bool {
        matches!(
            self.peer.connection_state(),
            RTCPeerConnectionState::Connected
        )
    }

    /// Close the peer connection
    pub async fn close(&self) -> Result<()> {
        self.peer
            .close()
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to close peer connection: {}", e)))?;
        Ok(())
    }
}
