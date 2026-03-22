mod interface;
pub use interface::{ToBackend, ToFrontend};

mod user;
pub use user::{Id as UserId, User};

pub mod store;
pub use store::Store;

mod identification;
pub use identification::Identification;

#[cfg(feature = "backend")]
#[path = "."]
mod b {
    use super::*;
    pub mod backend;
    //    pub mod database;
    //    pub use database::UserDatabase;
}

#[cfg(feature = "backend")]
pub use b::*;

//---- to remove
//#[cfg(any(feature = "desktop", feature = "mobile"))]
//pub mod init;

//#[cfg(any(feature = "desktop", feature = "mobile"))]
//pub mod manager;
