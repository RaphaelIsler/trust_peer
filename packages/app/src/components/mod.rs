
#[cfg(any(feature = "desktop", feature = "mobile"))]
mod user_manager;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use user_manager::UserManager;

#[cfg(any(feature = "desktop", feature = "mobile"))]
mod p2p_test;
#[cfg(any(feature = "desktop", feature = "mobile"))]
pub use p2p_test::P2PTestComponent;
