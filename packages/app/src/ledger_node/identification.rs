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
    use crate::i18n::{use_i18n, Key};
    use dioxus::prelude::*;

    #[component]
    pub fn Create(
        on_create: EventHandler<Identification>,
        on_error: EventHandler<String>,
    ) -> Element {
        let i18n = use_i18n();
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
                on_error.call(i18n.t(Key::FillRequiredFields).to_string());
                return;
            }

            let date = match chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                Ok(d) => d,
                Err(_) => {
                    on_error.call(i18n.t(Key::InvalidDateFormat).to_string());
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
            div { class: "form-card",
                h3 { "{i18n.t(Key::CreateNewIdentity)}" }

                div { class: "form-group",
                    label { class: "form-label", "{i18n.t(Key::FirstName)}" }
                    input {
                        class: "form-input",
                        r#type: "text",
                        value: "{first_name}",
                        oninput: move |e| first_name.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { class: "form-label", "{i18n.t(Key::LastName)}" }
                    input {
                        class: "form-input",
                        r#type: "text",
                        value: "{last_name}",
                        oninput: move |e| last_name.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { class: "form-label", "{i18n.t(Key::MiddleName)}" }
                    input {
                        class: "form-input",
                        r#type: "text",
                        value: "{middle_name}",
                        oninput: move |e| middle_name.set(e.value()),
                    }
                }
                div { class: "form-group",
                    label { class: "form-label", "{i18n.t(Key::DateOfBirth)}" }
                    input {
                        class: "form-input",
                        r#type: "date",
                        value: "{birth_date}",
                        oninput: move |e| birth_date.set(e.value()),
                    }
                }

                div { class: "form-actions",
                    button {
                        class: "btn btn--success",
                        onclick: handle_submit,
                        "{i18n.t(Key::CreateBtn)}"
                    }
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
                    div { class: "identification",
                        span { class: "identification__name", "{first_name} {last_name}" }
                        if let Some(mn) = &middle_name {
                            span { class: "identification__middle", "({mn})" }
                        }
                        span { class: "identification__birth", "· {birth_date}" }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "frontend")]
pub use m_frontend::*;
