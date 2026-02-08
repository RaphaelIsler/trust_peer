//! App - Unified Dioxus Fullstack application

pub mod components;
pub mod server_fn;

#[cfg(any(feature = "desktop", feature = "mobile"))]
pub mod user_init;

pub use components::*;
