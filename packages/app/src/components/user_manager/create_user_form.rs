use chrono::NaiveDate;
use dioxus::prelude::*;
use std::collections::HashMap;

use crate::{LedgerNode, User, UserDatabase};

#[component]
pub fn CreateUserForm(
    user_events: Signal<HashMap<String, Vec<crate::ledger_node::ui::LedgerEvent>>>,
    on_submit: EventHandler<(User, crate::ledger_node::ui::LedgerNode)>,
    on_error: EventHandler<String>,
) -> Element {
    let mut first_name = use_signal(|| String::new());
    let mut last_name = use_signal(|| String::new());
    let mut middle_name = use_signal(|| String::new());
    let mut birth_date = use_signal(|| String::new());

    let handle_submit = move |_| {
        spawn(async move {
            let first = first_name();
            let last = last_name();
            let middle = middle_name();
            let date_str = birth_date();

            if first.trim().is_empty() || last.trim().is_empty() || date_str.trim().is_empty() {
                on_error.call("Please fill in all required fields".to_string());
                return;
            }

            let date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    on_error.call("Invalid date format. Use YYYY-MM-DD".to_string());
                    return;
                }
            };

            let middle_opt = if middle.trim().is_empty() {
                None
            } else {
                Some(middle.trim().to_string())
            };

            match create_user(first, last, middle_opt, date).await {
                Ok((user, service, event_rx)) => {
                    let uid = user.id.inner().to_string();
                    let mut ue = user_events.clone();
                    spawn(async move {
                        let mut rx = event_rx;
                        while let Some(event) = rx.recv().await {
                            ue.write().entry(uid.clone()).or_default().push(event);
                        }
                    });
                    first_name.set(String::new());
                    last_name.set(String::new());
                    middle_name.set(String::new());
                    birth_date.set(String::new());
                    on_submit.call((user, service));
                }
                Err(e) => {
                    on_error.call(format!("Error creating user: {}", e));
                }
            }
        });
    };

    rsx! {
        div {
            class: "user-form",
            style: "background: #f5f5f5; padding: 20px; margin: 20px 0; border-radius: 8px;",

            h3 { "Create New User" }

            div { style: "margin: 10px 0;",
                label { style: "display: block; margin-bottom: 5px; font-weight: bold;",
                    "First Name *"
                }
                input {
                    r#type: "text",
                    value: "{first_name}",
                    oninput: move |e| first_name.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                    placeholder: "Max",
                }
            }

            div { style: "margin: 10px 0;",
                label { style: "display: block; margin-bottom: 5px; font-weight: bold;",
                    "Last Name *"
                }
                input {
                    r#type: "text",
                    value: "{last_name}",
                    oninput: move |e| last_name.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                    placeholder: "Mustermann",
                }
            }

            div { style: "margin: 10px 0;",
                label { style: "display: block; margin-bottom: 5px; font-weight: bold;",
                    "Middle Name (optional)"
                }
                input {
                    r#type: "text",
                    value: "{middle_name}",
                    oninput: move |e| middle_name.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                    placeholder: "Alexander",
                }
            }

            div { style: "margin: 10px 0;",
                label { style: "display: block; margin-bottom: 5px; font-weight: bold;",
                    "Date of Birth * (YYYY-MM-DD)"
                }
                input {
                    r#type: "date",
                    value: "{birth_date}",
                    oninput: move |e| birth_date.set(e.value()),
                    style: "width: 100%; padding: 8px; border: 1px solid #ccc; border-radius: 4px;",
                }
            }

            button {
                onclick: handle_submit,
                style: "padding: 10px 20px; background: #28a745; color: white; border: none; border-radius: 4px; cursor: pointer; margin-top: 10px;",
                "Create User"
            }
        }
    }
}

async fn create_user(
    first_name: String,
    last_name: String,
    middle_name: Option<String>,
    date_of_birth: NaiveDate,
) -> anyhow::Result<(
    User,
    LedgerNode,
    tokio::sync::mpsc::Receiver<crate::ledger_node::ui::LedgerEvent>,
)> {
    use crate::user_init::get_db_path;

    let db_path = get_db_path()?;
    let db = UserDatabase::open(&db_path).await?;
    Ok(db
        .create_user(first_name, last_name, middle_name, date_of_birth)
        .await?)
}
