pub type Id = blockchain::blockchain::Id;

mod config;
pub use config::{Config, WebRtcIds, WebRtcIdsInputView, WebRtcIdsShareView};

mod connection;
pub use connection::{Connection, ConnectionView};

mod state;
pub use state::{State, TrustState};

pub mod handler;
pub use handler::Service;
