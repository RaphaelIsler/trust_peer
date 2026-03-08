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

mod bank_account;
mod block_entry;
mod blockchain_validation;
mod error;
mod ledger_node;
mod peer_connection;
mod user;
mod user_db;
use money::Money;

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[path = "."]
mod backend {
    use super::*;
    pub mod user_init;
    pub use bank_account::BankAccountView;
    pub use components::*;
    pub use money::MoneyView;
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use backend::*;

pub use bank_account::{BankAccount, BankAccountId};
pub use block_entry::{AppBlock, BlockEntry};
pub use core_types::{KeyValue, Value};
pub use core_types::{TimestampU64View, TimestampView};
pub use error::{Result, UserError};
pub use ledger_node::ui::LedgerNode;
pub use user::{User, UserId};
pub use user_db::UserDatabase;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use blockchain_validation::{verify_money_with_state, MoneyValidationState};
pub use core_types::{Timestamp, Q32_32};
