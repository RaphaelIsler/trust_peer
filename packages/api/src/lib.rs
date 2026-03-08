use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToCoordinator {
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FromCoordinator {
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WakeupPlatform {
    Fcm,
    Apns,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeupRequest {
    pub platform: WakeupPlatform,
    pub device_token: String,
    pub title: Option<String>,
    pub body: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeupResponse {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
}

pub mod server_config;
pub use server_config::{ApnsConfig, FcmConfig, ServerConfig};
