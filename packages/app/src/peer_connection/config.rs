use anyhow::Result;
#[cfg(any(feature = "desktop", feature = "mobile"))]
use dioxus::prelude::*;
use p2p_webrtc::{PeerId, RoomId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Id = blockchain::blockchain::Id;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebRtcIds {
    pub own_id: PeerId,
    pub remote_id: PeerId,
    pub room_id: RoomId,
}

impl WebRtcIds {
    pub fn new() -> Self {
        Self {
            own_id: PeerId::from_uuid(Uuid::new_v4()),
            remote_id: PeerId::from_uuid(Uuid::new_v4()),
            room_id: RoomId::from_uuid(Uuid::new_v4()),
        }
    }

    pub fn to_share_text(&self) -> String {
        format!(
            "own_id={}\nremote_id={}\nroom_id={}",
            self.own_id, self.remote_id, self.room_id
        )
    }

    pub fn from_share_text(input: &str) -> Result<Self> {
        let mut own_id = None::<PeerId>;
        let mut remote_id = None::<PeerId>;
        let mut room_id = None::<RoomId>;

        for line in input.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let Some((key, value)) = trimmed.split_once('=') else {
                continue;
            };

            let key = key.trim();
            let value = value.trim();

            match key {
                "own_id" => {
                    own_id = Some(
                        PeerId::parse_str(value).map_err(|_| anyhow::anyhow!("Invalid own_id"))?,
                    )
                }
                "remote_id" => {
                    remote_id = Some(
                        PeerId::parse_str(value)
                            .map_err(|_| anyhow::anyhow!("Invalid remote_id"))?,
                    )
                }
                "room_id" => {
                    room_id = Some(
                        RoomId::parse_str(value).map_err(|_| anyhow::anyhow!("Invalid room_id"))?,
                    )
                }
                _ => {}
            }
        }

        let own_id = own_id.ok_or_else(|| anyhow::anyhow!("Missing own_id"))?;
        let remote_id = remote_id.ok_or_else(|| anyhow::anyhow!("Missing remote_id"))?;
        let room_id = room_id.ok_or_else(|| anyhow::anyhow!("Missing room_id"))?;

        Ok(Self {
            own_id,
            remote_id,
            room_id,
        })
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn WebRtcIdsShareView(ids: WebRtcIds) -> Element {
    let text = ids.to_share_text();

    rsx! {
        div { class: "webrtc-ids-share",
            div { class: "webrtc-ids-share__title", "WebRtcIds" }
            textarea {
                readonly: true,
                rows: "4",
                style: "width: 100%; font-family: monospace;",
                value: "{text}",
            }
            div {
                class: "webrtc-ids-share__qr-placeholder",
                style: "margin-top: 8px; padding: 12px; border: 1px dashed #999;",
                "QR-Code Platzhalter"
            }
        }
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn WebRtcIdsInputView(on_submit: EventHandler<WebRtcIds>) -> Element {
    let mut input_text = use_signal(String::new);
    let mut error_text = use_signal(|| None::<String>);

    rsx! {
        div { class: "webrtc-ids-input",
            div { class: "webrtc-ids-input__title", "WebRtcIds einlesen" }
            textarea {
                rows: "6",
                style: "width: 100%; font-family: monospace;",
                value: "{input_text}",
                placeholder: "own_id=...\nremote_id=...\nroom_id=...",
                oninput: move |event| input_text.set(event.value()),
            }
            button {
                style: "margin-top: 8px;",
                onclick: move |_| {
                    match WebRtcIds::from_share_text(&input_text()) {
                        Ok(ids) => {
                            error_text.set(None);
                            on_submit.call(ids);
                        }
                        Err(error) => {
                            error_text.set(Some(error.to_string()));
                        }
                    }
                },
                "Übernehmen"
            }
            if let Some(error) = error_text() {
                div { style: "margin-top: 8px; color: #b00020;", "{error}" }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Config {
    Tcp { ip: String },
    WebRtc { ids: WebRtcIds },
}
