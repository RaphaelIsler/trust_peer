use super::{Connection, WebRtcIds};
use serde::{Deserialize, Serialize};

/// Frontend state for a single peer connection, covering the full lifecycle:
/// pending setup → active → (failure handled externally via ErrorOverlay).
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionEntry {
    /// Connection is being established.
    /// `ids` is Some only for the initiating side (IDs to share via QR/text).
    /// `progress` carries the latest status update from the backend.
    Pending {
        new_connection_id: u8,
        ids: Option<WebRtcIds>,
        progress: Option<(String, f32)>,
    },
    /// Connection is fully established and backed by a `Connection` record.
    Active(Connection),
    /// Setup failed. Kept briefly so the UI can surface an error.
    Failed { new_connection_id: u8 },
}

impl ConnectionEntry {
    pub fn new_connection_id(&self) -> Option<u8> {
        match self {
            Self::Pending { new_connection_id, .. } => Some(*new_connection_id),
            Self::Failed { new_connection_id } => Some(*new_connection_id),
            Self::Active(_) => None,
        }
    }
}
