use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Identification {
    NameAndBirth {
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        birth_date: chrono::NaiveDate,
    },
    /*    Passport{number: String, expiration_date: Date},
    DriverLicense{number: String, expiration_date: Date},
    SocialSecurityNumber{number: String},
    PassportAndDriverLicense{passport_number: String, driver_license_number: String, expiration_date: Date},
    PassportAndSocialSecurityNumber{passport_number: String, social_security_number: String},
    DriverLicenseAndSocialSecurityNumber{driver_license_number: String, social_security_number: String},
    PassportAndDriverLicenseAndSocialSecurityNumber{passport_number: String, driver_license_number: String, social_security_number: String},}*/
}

pub enum IdentificationHash {
    NameAndBirth(String),
    /*    Passport(String),
    DriverLicense(String),
    SocialSecurityNumber(String),
    PassportAndDriverLicense(String),
    PassportAndSocialSecurityNumber(String),
    DriverLicenseAndSocialSecurityNumber(String),
    PassportAndDriverLicenseAndSocialSecurityNumber(String),*/
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
                        class: "user-info",
                        style: "background: #f5f5f5; padding: 20px; margin: 20px 0; border-radius: 8px;",
                        h3 { "User Information" }
                        p { "First Name: {first_name}" }
                        p { "Last Name: {last_name}" }
                        if let Some(mn) = &middle_name {
                            p { "Middle Name: {mn}" }
                        }
                        p { "Date of Birth: {birth_date}" }
                    }
                }
            }
        }
    }
}
#[cfg(feature = "frontend")]
pub use m_frontend::*;
