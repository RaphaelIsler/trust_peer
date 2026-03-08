//! P2P WebRTC Library
//!
//! A modern, async-first WebRTC library for mobile and desktop applications.
//! Provides a simple API for establishing peer-to-peer connections via WebSocket signaling.

pub mod api;
pub mod data_channel;
pub mod error;
pub mod peer;
pub mod signaling;

pub use api::{P2pConfig, P2pWebRtc, PeerId, RoomId};
pub use error::{Error, Result};
pub use peer::IceCandidate;

use log::info;

/// Initialize logging for the library
pub fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    info!("P2P WebRTC library initialized");
}
