use crate::peer_connection::{Connection, WebRtcIds};
use crate::AppBlock;
use serde::{Deserialize, Serialize};
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToFrontend {
    LedgerChains {
        private: blockchain::blockchain::Id,
        public: blockchain::blockchain::Id,
    },
    Blocks {
        id: blockchain::blockchain::Id,
        blocks: Vec<AppBlock>,
    },
    ConnectionsIds {
        new_connection_id: u8,
        ids: WebRtcIds,
    },
    ConnectionEstablished {
        new_connection_id: u8,
        connection: crate::peer_connection::Id,
    },
    ConnectionEstablishedFailed {
        new_connection_id: u8,
    },
    ConnectionLost {
        connection: crate::peer_connection::Id,
    },
    WaitForNameAccept {
        connection_id: blockchain::Id,
        identification: crate::user::Identification,
    },
    PeerConnections {
        connections: Vec<Connection>,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToBackend {
    GetPrivateAndPublic,
    GetBlocks {
        id: blockchain::blockchain::Id,
        count: usize,
        start_at: Option<usize>,
    },
    StartNewConnection {
        new_connection_id: u8,
    },
    CreateNewConnectionFromWebRtcIds {
        new_connection_id: u8,
        ids: WebRtcIds,
    },
    GetPeerConnections,
    UpdatePeerConnectionName {
        id: blockchain::blockchain::Id,
        name: Option<String>,
    },
}
