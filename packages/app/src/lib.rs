//! App - Unified Dioxus Fullstack application

pub mod components;
pub mod server_fn;
mod money;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod error;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod key_value;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_db;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_init;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_instance;

pub use components::*;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use error::{Result, UserError};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use key_value::{KeyValue, Value};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user::{User, UserId};
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_instance::Service;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_db::UserDatabase;
