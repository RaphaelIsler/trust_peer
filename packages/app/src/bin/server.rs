use dioxus::prelude::*;
use app::{Echo, Hero};

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[tokio::main]
async fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Hero {}
        Echo {}
    }
}
