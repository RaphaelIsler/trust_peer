use serde::{Deserialize, Serialize};
use dioxus::prelude::*;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize )]
pub enum WarningLevel{
    Ok,
    Warning,
    Critical,
}

impl Default for WarningLevel {
    fn default() -> Self {
        WarningLevel::Ok
    }
}

#[cfg(feature = "frontend")]
#[component]
pub fn WarningLevelView(
    level: WarningLevel,
) -> Element {
    let (text, color) = match level {
        WarningLevel::Ok => ("OK", "green"),
        WarningLevel::Warning => ("Warning", "orange"),
        WarningLevel::Critical => ("Critical", "red"),
    };
    return rsx!(
        span { style: "color: {color}; font-weight: bold;", "{text}" }
    );
}