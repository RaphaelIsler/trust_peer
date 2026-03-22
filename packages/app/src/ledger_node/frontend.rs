use super::{ToBackend, ToFrontend};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontend {
    current: crate::money::Money,
    private: blockchain::blockchain::Id,
    public: blockchain::blockchain::Id,
}

impl Frontend {
    pub fn new() -> Self {
        Self { current: crate::money::Money::default(), private: blockchain::blockchain::Id::new(), public: blockchain::blockchain::Id::new() }
    }

    pub fn handle_msg(&mut self, msg: ToFrontend) {
        match msg {
            ToFrontend::LedgerChains { private, public } =>{
                self.private = private;
                self.public = public;
            }
            _ => {}
        }
    }
}

#[cfg(feature = "frontend")]
#[path = "."]
mod m_frontend {
    use super::*;
    #[component]
    pub fn Overview(to_backend: EventHandler<ToBackend>, store: super::Frontend) -> Element {
        rsx! {
            div {
                class: "user-form",
                style: "background: #f5f5f5; padding: 20px; margin: 20px 0; border-radius: 8px;",
                crate::money::MoneyView {
                    money: store.current,
                    shown_amount: crate::money::MoneyViewMode::Current,
                }
            }
        }
    }
}
#[cfg(feature = "frontend")]
#[allow(unused_imports)]
pub use m_frontend::*;
