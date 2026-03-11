use dioxus::prelude::*;

use crate::{LedgerNode, User, UserDatabase};
use std::collections::HashMap;

mod create_user_form;
use create_user_form::CreateUserForm;

mod user_card;
use user_card::UserCard;

/// Komponente zur Anzeige und Verwaltung von Benutzern
#[component]
pub fn UserManager() -> Element {
    let mut users = use_signal(|| Vec::<User>::new());
    let mut show_form = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut user_services =
        use_signal(|| HashMap::<String, crate::ledger_node::ui::LedgerNode>::new());
    let user_events =
        use_signal(|| HashMap::<String, Vec<crate::ledger_node::ui::LedgerEvent>>::new());

    // Lade Benutzer beim ersten Render
    use_effect(move || {
        spawn(async move {
            if let Ok(base_path) = crate::get_data_directory() {
                match load_users().await {
                    Ok(user_list) => {
                        let mut services_map = user_services.write();
                        for user in &user_list {
                            let service =
                                match LedgerNode::start(base_path.clone(), &user.id).await {
                                    Ok((service, event_rx)) => {
                                        let uid = user.id.inner().to_string();
                                        let mut ue = user_events.clone();
                                        spawn(async move {
                                            let mut rx = event_rx;
                                            while let Some(event) = rx.recv().await {
                                                ue.write()
                                                    .entry(uid.clone())
                                                    .or_default()
                                                    .push(event);
                                            }
                                        });
                                        service
                                    }
                                    Err(e) => {
                                        error_message.set(Some(format!(
                                            "Fehler beim Starten des Service für {}: {e}",
                                            user.full_name()
                                        )));
                                        continue;
                                    }
                                };
                            services_map.insert(user.id.inner().to_string(), service);
                        }
                        users.set(user_list);
                    }
                    Err(e) => error_message.set(Some(format!("Fehler beim Laden: {}", e))),
                }
            }
        });
    });

    rsx! {
        div { class: "user-manager",
            h2 { "Benutzerverwaltung" }

            if let Some(err) = error_message() {
                div {
                    class: "error-message",
                    style: "color: red; padding: 10px; margin: 10px 0; border: 1px solid red; border-radius: 4px;",
                    "{err}"
                }
            }

            div { class: "user-actions", style: "margin: 20px 0;",
                button {
                    onclick: move |_| show_form.set(!show_form()),
                    style: "padding: 10px 20px; background: #007bff; color: white; border: none; border-radius: 4px; cursor: pointer;",
                    if show_form() {
                        "Close Form"
                    } else {
                        "Create New User"
                    }
                }
            }

            if show_form() {
                CreateUserForm {
                    user_events,
                    on_submit: move |(user, service): (User, crate::ledger_node::ui::LedgerNode)| {
                        let user_id = user.id.inner().to_string();
                        users.write().push(user);
                        user_services.write().insert(user_id, service);
                        show_form.set(false);
                        error_message.set(None);
                    },
                    on_error: move |err| {
                        error_message.set(Some(err));
                    },
                }
            }

            div { class: "user-list",
                h3 { "Registered Users ({users.read().len()})" }

                if users.read().is_empty() {
                    p { style: "color: #666; font-style: italic;",
                        "No users available. Create a new user."
                    }
                }

                for user in users.read().iter() {
                    if let Some(service) = user_services().get(&user.id.inner().to_string()).cloned() {
                        UserCard {
                            user: user.clone(),
                            service,
                        }
                    }
                }
            }
        }
    }
}

async fn load_users() -> anyhow::Result<Vec<User>> {
    use crate::user_init::get_db_path;

    let db_path = get_db_path()?;
    let db = UserDatabase::open(&db_path).await?;
    Ok(db.list_users().await?)
}
