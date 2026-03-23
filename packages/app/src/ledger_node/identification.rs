use serde::{Deserialize, Serialize};

/// Identification data for a LedgerNode owner.
/// Multiple identifications per LedgerNode are supported.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Identification {
    NameAndBirth {
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        birth_date: chrono::NaiveDate,
    },
}

impl Identification {
    pub fn display_name(&self) -> String {
        match self {
            Identification::NameAndBirth {
                first_name,
                last_name,
                ..
            } => format!("{} {}", first_name, last_name),
        }
    }

    /// Key for storing this identification in key_db by index.
    pub fn storage_key(index: u32) -> String {
        format!("identification.{}", index)
    }

    /// Key for storing the identification count in key_db.
    pub fn count_key() -> &'static str {
        "identification.count"
    }

    /// Serialize to JSON for key_db storage.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON.
    pub fn from_json(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

#[cfg(feature = "backend")]
impl Identification {
    /// SHA-256 hash via `crypto::Hash`. Stored as `BlockEntry::Identification { data }` in the public chain.
    pub fn hash_bytes(&self) -> Vec<u8> {
        let json = serde_json::to_string(self).unwrap_or_default();
        crypto::Hash::new(json.as_bytes()).0.to_vec()
    }
}

#[cfg(feature = "frontend")]
#[path = "."]
mod m_frontend {
    use super::*;
    use dioxus::prelude::*;

    #[component]
    pub fn Create(
        on_create: EventHandler<Identification>,
        on_error: EventHandler<String>,
    ) -> Element {
        let mut first_name = use_signal(|| String::new());
        let mut last_name = use_signal(|| String::new());
        let mut middle_name = use_signal(|| String::new());
        let mut birth_date = use_signal(|| String::new());

        let handle_submit = move |_| {
            let first = first_name();
            let last = last_name();
            let middle = middle_name();
            let date_str = birth_date();

            if first.trim().is_empty() || last.trim().is_empty() || date_str.trim().is_empty() {
                on_error.call("Please fill in all required fields".to_string());
                return;
            }

            let date = match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
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

            on_create.call(Identification::NameAndBirth {
                first_name: first,
                last_name: last,
                middle_name: middle_opt,
                birth_date: date,
            });
        };

        rsx! {
            div {
                class: "user-form",
                style: "background: #f5f5f5; padding: 20px; margin: 20px 0; border-radius: 8px;",

                h3 { "Create New Identity" }

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
                    "Create"
                }
            }
        }
    }

    #[component]
    pub fn Show(identification: Identification) -> Element {
        match identification {
            Identification::NameAndBirth {
                first_name,
                last_name,
                middle_name,
                birth_date,
            } => {
                rsx! {
                    div {
                        class: "identification",
                        style: "padding: 8px 0;",
                        span { "{first_name} {last_name}" }
                        if let Some(mn) = &middle_name {
                            span { style: "color: #666;", " ({mn})" }
                        }
                        span { style: "color: #666; font-size: 0.9em;", " · {birth_date}" }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "frontend")]
pub use m_frontend::*;
