pub type Id = blockchain::blockchain::Id;

mod config;
pub use config::{Config, WebRtcIds, WebRtcIdsInputView, WebRtcIdsShareView};

mod connection;
pub use connection::{Connection, ConnectionView};

pub mod handler;
pub use handler::Service;
