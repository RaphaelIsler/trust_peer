use dioxus::prelude::*;

#[cfg(any(feature = "desktop", feature = "mobile"))]
use crate::{User, UserDatabase};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use chrono::NaiveDate;

/// Komponente zur Anzeige und Verwaltung von Benutzern
#[component]
pub fn UserManager() -> Element {
    #[cfg(any(feature = "desktop", feature = "mobile"))]
    {
        let mut users = use_signal(|| Vec::<User>::new());
        let mut show_form = use_signal(|| false);
        let mut error_message = use_signal(|| None::<String>);

        // Lade Benutzer beim ersten Render
        use_effect(move || {
            spawn(async move {
                use crate::user_init::{get_db_path, start_all_user_services};

                if let Ok(db_path) = get_db_path() {
                    if let Err(e) = start_all_user_services(&db_path).await {
                        error_message
                            .set(Some(format!("Fehler beim Starten der Services: {}", e)));
                    }
                }

                match load_users().await {
                    Ok(user_list) => users.set(user_list),
                    Err(e) => error_message.set(Some(format!("Fehler beim Laden: {}", e))),
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
                    UserForm {
                        on_submit: move |user| {
                            users.write().push(user);
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
                        UserCard { user: user.clone() }
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
fn UserForm(on_submit: EventHandler<User>, on_error: EventHandler<String>) -> Element {
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
                Ok(user) => {
                    first_name.set(String::new());
                    last_name.set(String::new());
                    middle_name.set(String::new());
                    birth_date.set(String::new());
                    on_submit.call(user);
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
fn UserCard(user: User) -> Element {
    let mut show_details = use_signal(|| false);

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
) -> anyhow::Result<User> {
    use crate::user_init::get_db_path;

    let db_path = get_db_path()?;
    let db = UserDatabase::open(&db_path).await?;
    let user = db.create_user(first_name, last_name, middle_name, date_of_birth).await?;
    Ok(user)
}
