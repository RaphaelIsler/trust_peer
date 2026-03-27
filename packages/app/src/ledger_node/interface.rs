use crate::money::Money;
use crate::peer_connection::{Connection, WebRtcIds};
use crate::AppBlock;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToFrontend {
    Initialized {
        private: blockchain::blockchain::Id,
        public: blockchain::blockchain::Id,
        money: Money,
        identifications: Vec<super::Identification>,
        connection: crate::peer_connection::overview::Store,
    },
    Blocks {
        id: blockchain::blockchain::Id,
        blocks: Vec<AppBlock>,
    },
    Identifications(Vec<super::Identification>),
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
        identification: super::Identification,
    },
    PeerConnections {
        connections: Vec<Connection>,
    },
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToBackend {
    Init,
    GetBlocks {
        id: blockchain::blockchain::Id,
        count: usize,
        start_at: Option<usize>,
    },
    AddIdentification(super::Identification),
    GetIdentifications,
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
