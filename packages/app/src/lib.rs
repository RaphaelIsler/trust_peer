//! App - Unified Dioxus Fullstack application

pub mod components;
mod money;
pub mod server_fn;
use std::path::PathBuf;
/// Returns the data directory for the application
pub fn get_data_directory() -> anyhow::Result<PathBuf> {
    // Platform-specific data directories
    #[cfg(target_os = "android")]
    {
        // For Android: use the app-specific data directory
        // This would need to be retrieved from the Android system via JNI or similar
        // Fallback to a relative directory for development
        Ok(PathBuf::from("./data"))
    }

    #[cfg(not(target_os = "android"))]
    {
        // For Desktop: use the user directory
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"))?;
        let data_dir = PathBuf::from(home).join(".trust_peer").join("data");
        Ok(data_dir)
    }
}

mod interface;
pub use interface::{ToBackend, ToFrontend};
mod bank_account;
mod block_entry;
mod blockchain_validation;
mod error;
mod ledger_node;
mod peer_connection;
// user module kept as archive; not part of the main flow
// pub mod user;
use money::Money;

#[cfg(feature = "backend")]
#[path = "."]
mod b {
    use super::*;
    pub use bank_account::BankAccountView;
    pub use components::*;
    pub use money::MoneyView;
    pub mod backend;
}

#[cfg(feature = "backend")]
pub use b::*;

#[cfg(feature = "frontend")]
#[path = "."]
mod f {
    mod frontend;
    pub use frontend::App;
}

#[cfg(feature = "frontend")]
pub use f::*;

pub use bank_account::{BankAccount, BankAccountId};
pub use block_entry::{AppBlock, BlockEntry};
pub use core_types::{KeyValue, Value};
pub use core_types::{TimestampU64View, TimestampView};
pub use error::{Result, UserError};
//pub use ledger_node::ui::LedgerNode;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use blockchain_validation::{verify_money_with_state, MoneyValidationState};
pub use core_types::{Timestamp, Q32_32};
