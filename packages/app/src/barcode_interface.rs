use blockchain::blockchain::Id as BlockchainId;
use serde::{Deserialize, Serialize};

use crate::ledger_node;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Barcode {
    EstablishNewConnection{
        rtc: peer_connection::WebRtcIds,
    }
}
