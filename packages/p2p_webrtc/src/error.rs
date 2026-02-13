//! Error types for the P2P WebRTC library

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("WebRTC error: {0}")]
    WebRtc(String),

    #[error("Signaling error: {0}")]
    Signaling(String),

    #[error("DataChannel error: {0}")]
    DataChannel(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Not connected")]
    NotConnected,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, Error>;
