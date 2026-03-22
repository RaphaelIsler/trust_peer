use app::P2PTestComponent;
use dioxus::prelude::*;

use app::{App, ToBackend, ToFrontend};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    // Initialize logging
    let (tx_backend, rx_backend) = mpsc::channel::<ToBackend>(64);
    let (tx_frontend, rx_frontend) = mpsc::channel::<ToFrontend>(64);

    let base_path = app::get_data_directory().expect("failed to get data directory");
    app::backend::Service::start(base_path, rx_backend, tx_frontend);

    dioxus::LaunchBuilder::new()
        .with_context(tx_backend)
        .with_context(Arc::new(Mutex::new(Some(rx_frontend))))
        .launch(App);
}

/*#[component]
#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        UserManager {}
    }
}
*/
