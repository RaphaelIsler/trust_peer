use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToFrontend {
    Users(super::Store),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ToBackend {
    Load,
    Create(super::Identification),
}
