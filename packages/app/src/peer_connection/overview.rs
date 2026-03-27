use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use super::warning_level::WarningLevel;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct State{
    pub count: usize,
    pub warning_level: WarningLevel,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Store{
    pub strong: State,
    pub weak: State,
}

#[cfg(feature = "frontend")]
#[component]
pub fn Overview(
    store: Store,
) -> Element {
    return rsx!(
        div {
            span { "{store.strong.count}" }
            super::warning_level::WarningLevelView { level: store.strong.warning_level.clone() }
            span { " / {store.weak.count} " }
            super::warning_level::WarningLevelView { level: store.weak.warning_level.clone() }
        }
    );
}