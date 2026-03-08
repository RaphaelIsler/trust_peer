use app::UserManager;
use dioxus::prelude::*;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
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
