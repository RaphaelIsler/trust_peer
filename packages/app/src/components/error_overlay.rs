use dioxus::prelude::*;
use tokio::sync::mpsc;

pub type Errors = Signal<Vec<String>>;

/// Try-sends a message; on failure pushes a human-readable error into the overlay.
pub fn send_or_error<T>(tx: &mpsc::Sender<T>, msg: T, mut errors: Errors) {
    if let Err(e) = tx.try_send(msg) {
        errors.write().push(e.to_string());
    }
}

/// Global error overlay. Place once inside `App`, reads from the `Errors` context.
#[component]
pub fn ErrorOverlay() -> Element {
    let mut errors = use_context::<Errors>();

    if errors().is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "error-overlay",
            for (i, error) in errors().iter().cloned().enumerate() {
                div { class: "error-overlay__item",
                    span { class: "error-overlay__message", "{error}" }
                    button {
                        class: "error-overlay__dismiss",
                        onclick: move |_| { errors.write().remove(i); },
                        "×"
                    }
                }
            }
        }
    }
}
