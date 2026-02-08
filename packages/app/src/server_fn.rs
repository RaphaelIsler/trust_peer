//! Server functions - These run only on the server but are callable from the client
use dioxus::prelude::*;

/// Echo the user input on the server.
#[server]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}
