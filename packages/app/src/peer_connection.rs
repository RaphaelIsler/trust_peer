use anyhow::Result;
use p2p_webrtc::data_channel::DataChannel;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;
use p2p_webrtc::{PeerId, RoomId};
#[cfg(any(feature = "desktop", feature = "mobile"))]
use dioxus::prelude::*;

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
                        PeerId::parse_str(value)
                            .map_err(|_| anyhow::anyhow!("Invalid own_id"))?,
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
                        RoomId::parse_str(value)
                            .map_err(|_| anyhow::anyhow!("Invalid room_id"))?,
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
pub enum Transport {
    Tcp { ip: String },
    WebRtc { ids: WebRtcIds },
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeerConnection {
    pub id: Id,
    pub name: Option<String>,
    pub transport: Transport,
    #[serde(skip)]
    pub data_channel: Option<DataChannel>,
}

impl PartialEq for PeerConnection {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.transport == other.transport
    }
}

impl PeerConnection {
    pub fn new_web_rtc(id: Id) -> Self {
        Self {
            id,
            name: None,
            transport: Transport::WebRtc {
                ids: WebRtcIds::new(),
            },
            data_channel: None,
        }
    }

    pub fn new_tcp(id: Id, ip: String) -> Self {
        Self {
            id,
            name: None,
            transport: Transport::Tcp { ip },
            data_channel: None,
        }
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn PeerConnectionView(
    connection: PeerConnection,
    on_save_name: EventHandler<(Id, Option<String>)>,
) -> Element {
    let mut name_input = use_signal(|| connection.name.clone().unwrap_or_default());
    let connection_id = connection.id.inner().to_string();

    let transport_text = match &connection.transport {
        Transport::Tcp { ip } => format!("TCP ({ip})"),
        Transport::WebRtc { .. } => "WebRTC".to_string(),
    };

    rsx! {
        div { style: "border: 1px solid #ddd; border-radius: 6px; padding: 10px; margin: 8px 0;",

            div { style: "margin-bottom: 6px; font-family: monospace; font-size: 0.9em;",
                "{connection_id}"
            }
            div { style: "margin-bottom: 8px; color: #666;", "{transport_text}" }
            div { style: "display: flex; gap: 8px; align-items: center;",
                input {
                    r#type: "text",
                    value: "{name_input}",
                    placeholder: "Verbindungsname",
                    oninput: move |event| name_input.set(event.value()),
                    style: "flex: 1; padding: 6px; border: 1px solid #ccc; border-radius: 4px;",
                }
                button {
                    onclick: {
                        let id = connection.id.clone();
                        move |_| {
                            let trimmed = name_input().trim().to_string();
                            let next_name = if trimmed.is_empty() { None } else { Some(trimmed) };
                            on_save_name.call((id.clone(), next_name));
                        }
                    },
                    style: "padding: 6px 10px; background: #0d6efd; color: white; border: none; border-radius: 4px; cursor: pointer;",
                    "Speichern"
                }
            }
        }
    }
}

pub type UserConnection = PeerConnection;

#[derive(Clone)]
pub enum RuntimeEvent {
    DataReceived { connection_id: Id, data: Vec<u8> },
    Closed { connection_id: Id },
    Failed { connection_id: Id, error: String },
}

//#[derive(helper::ServiceWrapper)]
pub enum RuntimeMsg {
    Send {
        data: Vec<u8>,
        tx: oneshot::Sender<Result<()>>,
    },
    Stop {
        tx: oneshot::Sender<Result<()>>,
    },
}

#[derive(Clone)]
pub struct RuntimeService {
    connection_id: Id,
    tx: mpsc::Sender<RuntimeMsg>,
}

impl PartialEq for RuntimeService {
    fn eq(&self, other: &Self) -> bool {
        self.connection_id == other.connection_id && self.tx.same_channel(&other.tx)
    }
}

impl RuntimeService {
    pub fn connection_id(&self) -> &Id {
        &self.connection_id
    }

    pub fn start(connection_id: Id, data_channel: DataChannel, events_tx: mpsc::Sender<RuntimeEvent>) -> Self {
        let (tx, rx) = mpsc::channel(64);
        let task_connection_id = connection_id.clone();

        tokio::spawn(async move {
            Self::process(task_connection_id, data_channel, events_tx, rx).await;
        });

        Self { connection_id, tx }
    }

    async fn process(
        connection_id: Id,
        data_channel: DataChannel,
        events_tx: mpsc::Sender<RuntimeEvent>,
        mut rx: mpsc::Receiver<RuntimeMsg>,
    ) {
        let poll_interval = Duration::from_millis(50);

        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    match msg {
                        RuntimeMsg::Send { data, tx } => {
                            let result = data_channel
                                .send(&data)
                                .await
                                .map_err(|error| anyhow::anyhow!(error.to_string()));

                            if let Err(error) = &result {
                                let _ = events_tx.send(RuntimeEvent::Failed {
                                    connection_id: connection_id.clone(),
                                    error: error.to_string(),
                                }).await;
                            }

                            let _ = tx.send(result);
                        }
                        RuntimeMsg::Stop { tx } => {
                            let close_result = data_channel
                                .close()
                                .await
                                .map_err(|error| anyhow::anyhow!(error.to_string()));
                            let _ = tx.send(close_result);
                            let _ = events_tx.send(RuntimeEvent::Closed {
                                connection_id: connection_id.clone(),
                            }).await;
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(poll_interval) => {
                    if !data_channel.is_open() {
                        let _ = events_tx.send(RuntimeEvent::Closed {
                            connection_id: connection_id.clone(),
                        }).await;
                        break;
                    }

                    if let Some(data) = data_channel.try_recv().await {
                        let _ = events_tx.send(RuntimeEvent::DataReceived {
                            connection_id: connection_id.clone(),
                            data,
                        }).await;
                    }
                }
                else => {
                    break;
                }
            }
        }
    }
}

impl db::DbEntity for PeerConnection {
    type Id = Id;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn table_name() -> &'static str {
        "user_connections"
    }

    fn schema_version() -> u32 {
        3
    }

    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS user_connections (
                id TEXT PRIMARY KEY,
                name TEXT,
                transport_type TEXT NOT NULL,
                tcp_ip TEXT,
                web_rtc_own_id TEXT NOT NULL,
                web_rtc_remote_id TEXT NOT NULL,
                web_rtc_room_id TEXT NOT NULL
            );",
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn update_table(conn: &SqlitePool, from_version: u32, _to_version: u32) -> Result<()> {
        if from_version < 2 {
            let _ = sqlx::query("ALTER TABLE user_connections ADD COLUMN transport_type TEXT")
                .execute(conn)
                .await;
            let _ = sqlx::query("ALTER TABLE user_connections ADD COLUMN tcp_ip TEXT")
                .execute(conn)
                .await;
            sqlx::query(
                "UPDATE user_connections
                 SET transport_type = COALESCE(transport_type, 'webrtc')",
            )
            .execute(conn)
            .await?;
        }
        if from_version < 3 {
            let _ = sqlx::query("ALTER TABLE user_connections ADD COLUMN name TEXT")
                .execute(conn)
                .await;
        }
        Ok(())
    }

    async fn write(&self, conn: &SqlitePool) -> Result<()> {
        let id = self.id.clone();

        let (transport_type, tcp_ip, own_id, remote_id, room_id) = match &self.transport {
            Transport::Tcp { ip } => (
                "tcp",
                Some(ip.clone()),
                String::new(),
                String::new(),
                String::new(),
            ),
            Transport::WebRtc { ids } => (
                "webrtc",
                None,
                ids.own_id.to_string(),
                ids.remote_id.to_string(),
                ids.room_id.to_string(),
            ),
        };

        sqlx::query(
            "INSERT OR REPLACE INTO user_connections (
                id,
                name,
                transport_type,
                tcp_ip,
                web_rtc_own_id,
                web_rtc_remote_id,
                web_rtc_room_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(id)
        .bind(self.name.clone())
        .bind(transport_type)
        .bind(tcp_ip)
        .bind(own_id)
        .bind(remote_id)
        .bind(room_id)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();
        let row = sqlx::query(
            "SELECT id, name, transport_type, tcp_ip, web_rtc_own_id, web_rtc_remote_id, web_rtc_room_id
             FROM user_connections
             WHERE id = ?1",
        )
            .bind(&id)
            .fetch_optional(conn)
            .await?;

        if let Some(row) = row {
            let transport_type: String = row.try_get(2)?;
            let transport = match transport_type.as_str() {
                "tcp" => Transport::Tcp {
                    ip: row.try_get::<Option<String>, _>(3)?.unwrap_or_default(),
                },
                "webrtc" => Transport::WebRtc {
                    ids: WebRtcIds {
                        own_id: PeerId::parse_str(row.try_get::<String, _>(4)?.as_str())
                            .map_err(|_| anyhow::anyhow!("Invalid web_rtc_own_id in DB"))?,
                        remote_id: PeerId::parse_str(row.try_get::<String, _>(5)?.as_str())
                            .map_err(|_| anyhow::anyhow!("Invalid web_rtc_remote_id in DB"))?,
                        room_id: RoomId::parse_str(row.try_get::<String, _>(6)?.as_str())
                            .map_err(|_| anyhow::anyhow!("Invalid web_rtc_room_id in DB"))?,
                    },
                },
                other => anyhow::bail!("Unknown connection transport type '{}'.", other),
            };

            Ok(Some(Self {
                id: row.try_get(0)?,
                name: row.try_get(1)?,
                transport,
                data_channel: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM user_connections WHERE id = ?1")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query(
            "SELECT id, name, transport_type, tcp_ip, web_rtc_own_id, web_rtc_remote_id, web_rtc_room_id
             FROM user_connections
             ORDER BY id",
        )
            .fetch_all(conn)
            .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let transport_type: String = row.try_get(2)?;
            let transport = match transport_type.as_str() {
                "tcp" => Transport::Tcp {
                    ip: row.try_get::<Option<String>, _>(3)?.unwrap_or_default(),
                },
                "webrtc" => Transport::WebRtc {
                    ids: WebRtcIds {
                        own_id: PeerId::parse_str(row.try_get::<String, _>(4)?.as_str())
                            .map_err(|_| anyhow::anyhow!("Invalid web_rtc_own_id in DB"))?,
                        remote_id: PeerId::parse_str(row.try_get::<String, _>(5)?.as_str())
                            .map_err(|_| anyhow::anyhow!("Invalid web_rtc_remote_id in DB"))?,
                        room_id: RoomId::parse_str(row.try_get::<String, _>(6)?.as_str())
                            .map_err(|_| anyhow::anyhow!("Invalid web_rtc_room_id in DB"))?,
                    },
                },
                other => anyhow::bail!("Unknown connection transport type '{}'.", other),
            };

            items.push(Self {
                id: row.try_get(0)?,
                name: row.try_get(1)?,
                transport,
                data_channel: None,
            });
        }

        Ok(items)
    }
}