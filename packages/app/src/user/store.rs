use super::ToBackend;
use super::User;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Store {
    pub users: Vec<User>,
}

impl Store {
    pub fn new() -> Self {
        Self { users: vec![] }
    }

    pub fn from(users: Vec<User>) -> Self {
        Self { users }
    }

    pub fn is_empty(&self) -> bool {
        self.users.len() == 0
    }

    pub fn merge(&mut self, other: Self) {
        for user in other.users {
            if !self.users.iter().any(|u| u.id == user.id) {
                self.users.push(user);
            }
        }
    }
}

#[cfg(feature = "frontend")]
#[path = "."]
mod m_frontend {
    use super::*;
    #[component]
    pub fn Overview(to_backend: EventHandler<ToBackend>, store: super::Store) -> Element {
        rsx! {
            div {
                class: "user-form",
                style: "background: #f5f5f5; padding: 20px; margin: 20px 0; border-radius: 8px;",
                if store.is_empty() {
                    crate::user::identification::Create {
                        on_create: move |ident| {
                            to_backend.call(ToBackend::Create(ident).into());
                        },
                        on_error: move |_err| {},
                    }
                } else {
                    for user in store.users.iter() {
                        crate::user::user::Show { user: user.clone() }
                    }
                }
            }
        }
    }
}
#[cfg(feature = "frontend")]
#[allow(unused_imports)]
pub use m_frontend::*;
