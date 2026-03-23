use super::{identification::Identification, ToBackend, ToFrontend};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontend {
    pub current: crate::money::Money,
    pub private: blockchain::blockchain::Id,
    pub public: blockchain::blockchain::Id,
    pub identifications: Vec<Identification>,
}

impl Frontend {
    pub fn new() -> Self {
        Self {
            current: crate::money::Money::default(),
            private: blockchain::blockchain::Id::new(),
            public: blockchain::blockchain::Id::new(),
            identifications: vec![],
        }
    }

    pub fn display_name(&self) -> String {
        self.identifications
            .first()
            .map(|i| i.display_name())
            .unwrap_or_else(|| self.private.to_string())
    }

    pub fn handle_msg(&mut self, msg: ToFrontend) {
        match msg {
            ToFrontend::LedgerChains { private, public } => {
                self.private = private;
                self.public = public;
            }
            ToFrontend::Identifications(identifications) => {
                self.identifications = identifications;
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
                class: "ledger-node",
                style: "background: #f5f5f5; padding: 20px; margin: 20px 0; border-radius: 8px;",
                h3 { "{store.display_name()}" }
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
