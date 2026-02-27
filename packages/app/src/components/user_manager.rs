use dioxus::prelude::*;

#[cfg(any(feature = "desktop", feature = "mobile"))]
use crate::{Service, User, UserDatabase};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use chrono::NaiveDate;
#[cfg(any(feature = "desktop", feature = "mobile"))]
use blockchain::Block;
#[cfg(any(feature = "desktop", feature = "mobile"))]
use blockchain::BlockView;
#[cfg(any(feature = "desktop", feature = "mobile"))]
use std::collections::HashMap;

/// Komponente zur Anzeige und Verwaltung von Benutzern
#[component]
pub fn UserManager() -> Element {
    #[cfg(any(feature = "desktop", feature = "mobile"))]
    {
        let mut users = use_signal(|| Vec::<User>::new());
        let mut show_form = use_signal(|| false);
        let mut error_message = use_signal(|| None::<String>);
        let mut user_services = use_signal(|| HashMap::<String, crate::user_service::Service>::new());
        // Lade Benutzer beim ersten Render
        use_effect(move || {
            spawn(async move {
                if let Ok(base_path) = crate::get_data_directory() {
                    match load_users().await {
                        Ok(user_list) => {
                            let mut services_map = user_services.write();
                            for user in &user_list {
                                let service = match Service::start(base_path.clone(), &user.id).await {
                                    Ok(service) => service,
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
                        on_submit: move |(user, service): (User, crate::user_service::Service)| {
                            let user_id = user.id.inner().to_string();
                            users.write().push(user);
                            user_services
                                .write()
                                .insert(user_id, service);
                            let mut user_services = user_services.clone();
                            let mut error_message = error_message.clone();
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
                        UserCard {
                            user: user.clone(),
                            service: user_services().get(&user.id.inner().to_string()).cloned().unwrap(),
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(any(feature = "desktop", feature = "mobile")))]
    {
        rsx! {
            div {
                p { "User management is only available for Desktop and Mobile." }
            }
        }
    }
}

/// Form for creating a new user
#[component]
fn CreateUserForm(on_submit: EventHandler<(User, crate::user_service::Service)>, on_error: EventHandler<String>) -> Element {
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
                Ok((user, service)) => {
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

/// Card for displaying a user
#[component]
fn UserCard(user: User, service: Service) -> Element {
    let mut show_details = use_signal(|| false);
    let mut selected_chain = use_signal(|| None::<blockchain::blockchain::Id>);
    let mut private_id = use_signal(|| None::<blockchain::blockchain::Id>);
    let mut public_id = use_signal(|| None::<blockchain::blockchain::Id>);
    let mut active_chain = use_signal(|| None::<Vec<Block>>);
    let mut chain_error = use_signal(|| None::<String>);
    let mut chain_loading = use_signal(|| false);

    if private_id().is_none() {
        let service = service.clone();
        spawn(async move {
            match service.get_private_and_public().await {
                Ok((p_id, pub_id)) => {private_id.set(Some(p_id)); public_id.set(Some(pub_id));},
                Err(e) => chain_error.set(Some(format!("Failed to load chain IDs: {e}"))),
            }
        });
    }

/*    spawn(async move {
        chain.set(service.get_private_and_public().await.unwrap_or_else(|e| {
            chain_error.set(Some(format!("Failed to load blocks: {e}")));
            Vec::new()
        }));
        selected_chain.set(Some(selection));
        chain_loading.set(false);
    });
*/
    let chain_error_view = chain_error().map(|err| {
        rsx! {
            div { style: "margin: 10px 0; color: #b00020;", "{err}" }
        }
    });

    let chain_loading_view = if chain_loading() {
        Some(rsx! {
            div { style: "margin: 10px 0; color: #666;", "Loading blockchain..." }
        })
    } else {
        None
    };

    let chain_view = match active_chain(){
        Some(blocks) => Some(rsx! {
            div { class: "blockchain",
                if blocks.is_empty() {
                    div { class: "blockchain__empty", "No blocks" }
                }
                for (index , block) in blocks.into_iter().enumerate() {
                    BlockView { index, block }
                }
            }
        }),
        None => None
    };



    rsx! {
        div {
            class: "user-card",
            style: "border: 1px solid #ddd; padding: 15px; margin: 10px 0; border-radius: 8px; background: white;",

            div { style: "display: flex; justify-content: space-between; align-items: center;",
                div {
                    h4 { style: "margin: 0 0 5px 0;", "{user.full_name()}" }
                }
                button {
                    onclick: move |_| show_details.set(!show_details()),
                    style: "padding: 5px 15px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer;",
                    if show_details() {
                        "Less"
                    } else {
                        "Details"
                    }
                }
            }

            if show_details() {
                div { style: "margin-top: 15px; padding-top: 15px; border-top: 1px solid #eee;",

                    div { style: "margin: 10px 0;",
                        strong { "ID: " }
                        span { style: "font-family: monospace; font-size: 0.9em;", "{user.id:?}" }
                    }

                    div { style: "margin: 10px 0;",
                        strong { "Created at:" }
                        span { style: "font-size: 0.9em;", "{user.created_at.to_rfc2822()}" }
                    }

                    div { style: "margin: 16px 0; display: flex; gap: 8px;",
                        button {
                            onclick: {
                                let user_id = user.id.clone();
                                let active_chain = active_chain.clone();
                                let selected_chain = selected_chain.clone();
                                let chain_loading = chain_loading.clone();
                                let chain_error = chain_error.clone();
                                let service = service.clone();
                                move |_| select_chain(
                                    private_id(),
                                    active_chain,
                                    selected_chain,
                                    chain_loading,
                                    chain_error,
                                    service.clone(),
                                )
                            },
                            style: "padding: 6px 12px; background: #0d6efd; color: white; border: none; border-radius: 4px; cursor: pointer;",
                            "Private Chain"
                        }
                        button {
                            onclick: {
                                let user_id = user.id.clone();
                                let active_chain = active_chain.clone();
                                let selected_chain = selected_chain.clone();
                                let chain_loading = chain_loading.clone();
                                let chain_error = chain_error.clone();
                                let service = service.clone();
                                move |_| select_chain(
                                    public_id(),
                                    active_chain,
                                    selected_chain,
                                    chain_loading,
                                    chain_error,
                                    service.clone(),
                                )
                            },
                            style: "padding: 6px 12px; background: #6610f2; color: white; border: none; border-radius: 4px; cursor: pointer;",
                            "Public Chain"
                        }
                    }

                    {chain_error_view}
                    {chain_loading_view}
                    {chain_view}
                }
            }
        }
    }
}

// Helper functions for database operations
#[cfg(any(feature = "desktop", feature = "mobile"))]
async fn load_users() -> anyhow::Result<Vec<User>> {
    use crate::user_init::get_db_path;

    let db_path = get_db_path()?;
    let db = UserDatabase::open(&db_path).await?;
    let users = db.list_users().await?;
    Ok(users)
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
async fn create_user(
    first_name: String,
    last_name: String,
    middle_name: Option<String>,
    date_of_birth: NaiveDate,
) -> anyhow::Result<(User, Service)> {
    use crate::user_init::get_db_path;

    let db_path = get_db_path()?;
    let db = UserDatabase::open(&db_path).await?;
    Ok(db.create_user(first_name, last_name, middle_name, date_of_birth).await?)
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
fn select_chain(
    selection: Option<blockchain::blockchain::Id>,
    mut chain: Signal<Option<Vec<Block>>>,
    mut selected_chain: Signal<Option<blockchain::blockchain::Id>>,
    mut chain_loading: Signal<bool>,
    mut chain_error: Signal<Option<String>>,
    service: crate::user_service::Service,
) {
    if selection == selected_chain() {
        return;
    }
    if let Some(selection) = selection{

        chain_loading.set(true);
        chain_error.set(None);

        spawn(async move {
            chain.set(Some(service.get_blocks(selection, 100, None).await.unwrap_or_else(|e| {
                chain_error.set(Some(format!("Failed to load blocks: {e}")));
                Vec::new()
            })));
            selected_chain.set(Some(selection));
            chain_loading.set(false);
        });
    }
}
