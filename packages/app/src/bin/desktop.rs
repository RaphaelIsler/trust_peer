use app::{P2PTestComponent, UserManager};
use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    dioxus::launch(App);
}

#[component]
#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        UserManager {}
    }
}
