use dioxus::prelude::*;
use app::{Echo, Hero, UserManager};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // Initialisiere Benutzerdatenbank beim Start
    if let Err(e) = init_database() {
        eprintln!("Fehler beim Initialisieren der Datenbank: {}", e);
    }

    dioxus::launch(App);
}

fn init_database() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let db_path = app::user_init::init_user_database().await?;
        app::user_init::ensure_test_user(&db_path).await?;
        Ok(())
    })
}

#[component]
#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Hero {}
        UserManager {}
        Echo {}
    }
}
