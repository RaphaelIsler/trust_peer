//! App - Unified Dioxus Fullstack application

pub mod components;
pub mod server_fn;
mod money;
pub mod q32_32;
pub mod timestamp;
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
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))?;
        let data_dir = PathBuf::from(home)
            .join(".trust_peer")
            .join("data");
        Ok(data_dir)
    }
}


#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod error;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod key_value;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_db;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_connection;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod bank_account;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_init;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_service;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod block_entry;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod blockchain_validation;

pub use components::*;
pub use money::Money;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use money::MoneyView;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use timestamp::{TimestampU64View, TimestampView};

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use error::{Result, UserError};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use key_value::{KeyValue, Value};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user::{User, UserId};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_connection::{Transport as UserConnectionTransport, UserConnection, WebRtcIds};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_connection::{WebRtcIdsInputView, WebRtcIdsShareView};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use bank_account::{BankAccount, BankAccountId, BankAccountView};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_service::Service;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_db::UserDatabase;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use block_entry::{AppBlock, BlockEntry};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use blockchain_validation::{verify_money_with_state, MoneyValidationState};
