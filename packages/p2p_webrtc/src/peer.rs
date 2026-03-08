//! WebRTC peer connection module
//!
//! Manages the WebRTC PeerConnection, including SDP offer/answer exchange,
//! ICE candidate handling, and connection state management.

use crate::error::{Error, Result};
use log::{debug, info};
use std::sync::Arc;
use tokio::sync::mpsc;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

/// ICE candidate information
#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: String,
    pub sdp_mline_index: u32,
}

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
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: vec![],
        }
    }
}

/// WebRTC peer connection wrapper
pub struct PeerConnection {
    peer: Arc<RTCPeerConnection>,
    is_initiator: bool,
    ice_tx: mpsc::UnboundedSender<Option<IceCandidate>>,
}

impl PeerConnection {
    /// Create a new WebRTC peer connection
    /// Returns (PeerConnection, ICE candidate receiver)
    pub async fn new(
        is_initiator: bool,
        ice_config: IceServersConfig,
    ) -> Result<(Self, mpsc::UnboundedReceiver<Option<IceCandidate>>)> {
        info!(
            "Creating WebRTC peer connection (initiator: {})",
            is_initiator
        );

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

        let peer = Arc::new(peer);
        let (ice_tx, ice_rx) = mpsc::unbounded_channel();

        // Register ICE candidate callback
        {
            let ice_tx_clone = ice_tx.clone();
            peer.on_ice_candidate(Box::new(move |candidate| {
                if let Some(candidate) = candidate {
                    debug!("ICE candidate discovered: {} {}", candidate.foundation, candidate.address);
                    // Construct candidate string from components
                    let candidate_type = match candidate.typ {
                        webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Host => "host",
                        webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Srflx => "srflx",
                        webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Prflx => "prflx",
                        webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Relay => "relay",
                        webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Unspecified => "unknown",
                    };
                    let candidate_str = format!(
                        "candidate:{} {} {} {} {} {} typ {}",
                        candidate.foundation,
                        candidate.component,
                        candidate.protocol.to_string(),
                        candidate.priority,
                        candidate.address,
                        candidate.port,
                        candidate_type
                    );
                    let ice_candidate = IceCandidate {
                        candidate: candidate_str,
                        sdp_mid: String::new(),
                        sdp_mline_index: 0,
                    };
                    let _ = ice_tx_clone.send(Some(ice_candidate));
                } else {
                    debug!("ICE candidate gathering complete");
                    let _ = ice_tx_clone.send(None);
                }
                Box::pin(async {})
            }));
        }

        Ok((
            Self {
                peer,
                is_initiator,
                ice_tx,
            },
            ice_rx,
        ))
    }

    /// Create an SDP offer (initiator only)
    pub async fn create_offer(&self) -> Result<String> {
        if !self.is_initiator {
            return Err(Error::WebRtc("Only initiator can create offer".to_string()));
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

        debug!("Answer done");

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

        let mline_index = u16::try_from(sdp_mline_index).map_err(|_| {
            Error::WebRtc(format!(
                "Invalid sdp_mline_index (out of range): {}",
                sdp_mline_index
            ))
        })?;

        let init = RTCIceCandidateInit {
            candidate: candidate.to_string(),
            sdp_mid: if sdp_mid.is_empty() {
                None
            } else {
                Some(sdp_mid.to_string())
            },
            sdp_mline_index: Some(mline_index),
            username_fragment: None,
        };

        self.peer
            .add_ice_candidate(init)
            .await
            .map_err(|e| Error::WebRtc(format!("Failed to add ICE candidate: {}", e)))?;

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
