use blockchain::{block, blockchain::Id as BlockchainId};
use serde::{Deserialize, Serialize};

use crate::ledger_node;
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToFrontend {
    User(crate::user::ToFrontend),
    Ledger {
        private: BlockchainId,
        msg: ledger_node::ToFrontend,
    },
    Init(Vec<(crate::user::User, blockchain::blockchain::Id, crate::Money)>)
}

impl From<crate::user::ToFrontend> for ToFrontend {
    fn from(value: crate::user::ToFrontend) -> Self {
        Self::User(value)
    }
}

//-------------------
// Backend part

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToBackend {
    Init,
    User(crate::user::ToBackend),
    Ledger {
        private: BlockchainId,
        msg: ledger_node::ToBackend,
    },
}

impl From<crate::user::ToBackend> for ToBackend {
    fn from(value: crate::user::ToBackend) -> Self {
        Self::User(value)
    }
}
