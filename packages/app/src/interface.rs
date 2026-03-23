use blockchain::blockchain::Id as BlockchainId;
use serde::{Deserialize, Serialize};

use crate::ledger_node;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToFrontend {
    Ledger {
        private: BlockchainId,
        msg: ledger_node::ToFrontend,
    },
    /// Sent after Init or after creating/loading LedgerNodes.
    /// Each entry: (private chain id, current balance).
    Init(Vec<(BlockchainId, crate::Money)>),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToBackend {
    Init,
    /// Create a new LedgerNode with the given initial identification.
    Create(crate::ledger_node::Identification),
    Ledger {
        private: BlockchainId,
        msg: ledger_node::ToBackend,
    },
}
