use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = "server_config.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fcm: Option<FcmConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apns: Option<ApnsConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FcmConfig {
    pub service_account_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApnsConfig {
    pub key_id: String,
    pub team_id: String,
    pub bundle_id: String,
    pub private_key_path: String,
    #[serde(default)]
    pub use_sandbox: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".to_string(),
            port: 7001,
            fcm: None,
            apns: None,
        }
    }
}

#[derive(Debug)]
pub enum ServerConfigError {
    Io(std::io::Error),
    Serde(serde_json::Error),
}

impl std::fmt::Display for ServerConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerConfigError::Io(err) => write!(f, "io error: {err}"),
            ServerConfigError::Serde(err) => write!(f, "serde error: {err}"),
        }
    }
}

impl std::error::Error for ServerConfigError {}

impl From<std::io::Error> for ServerConfigError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for ServerConfigError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl ServerConfig {
    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self, ServerConfigError> {
        let path = path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let config = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ServerConfigError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn config_path_from_env() -> PathBuf {
        std::env::var("SERVER_CONFIG_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_CONFIG_PATH))
    }

    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}
