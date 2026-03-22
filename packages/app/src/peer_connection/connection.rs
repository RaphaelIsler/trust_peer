use anyhow::Result;
#[cfg(any(feature = "desktop", feature = "mobile"))]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use super::Id;
use super::Service;
use super::{Config, WebRtcIds};

#[derive(Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Id,
    pub name: Option<String>,
    pub config: Config,
    #[serde(skip)]
    pub handler: Option<Service>,
}

impl PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.config == other.config
    }
}

impl Connection {
    pub fn new_web_rtc(id: Id) -> Self {
        Self {
            id,
            name: None,
            config: Config::WebRtc {
                ids: WebRtcIds::new(),
            },
            handler: None,
        }
    }

    pub fn new_tcp(id: Id, ip: String) -> Self {
        Self {
            id,
            name: None,
            config: Config::Tcp { ip },
            handler: None,
        }
    }

    pub fn from_service(id: Id, webrtc: WebRtcIds, handler: Service) -> Self {
        Self {
            id,
            name: None,
            config: Config::WebRtc { ids: webrtc },
            handler: Some(handler),
        }
    }
}

impl Connection {
    pub fn is_active(&self) -> bool {
        self.handler.is_some()
    }

    pub fn start(&mut self, to_ledger: crate::ledger_node::backend::con::Service) {
        if self.handler.is_none() {
            if let Config::WebRtc { ids } = &self.config {
                let handler = Service::restart(self.id.clone(), ids.clone(), to_ledger);
                self.handler = Some(handler);
            }
        }
    }

    pub async fn stop(&mut self) {
        if let Some(handler) = &self.handler {
            let handler: &Service = handler;
            handler.stop().await;
        }
    }
    pub fn stopped(&mut self) {
        self.handler = None;
    }

    pub fn get_id(&self) -> &Id {
        &self.id
    }
}

impl db::DbEntity for Connection {
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
                config_type TEXT NOT NULL,
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
            let _ = sqlx::query("ALTER TABLE user_connections ADD COLUMN config_type TEXT")
                .execute(conn)
                .await;
            let _ = sqlx::query("ALTER TABLE user_connections ADD COLUMN tcp_ip TEXT")
                .execute(conn)
                .await;
            sqlx::query(
                "UPDATE user_connections
                 SET config_type = COALESCE(config_type, 'webrtc')",
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

        let (config_type, tcp_ip, own_id, remote_id, room_id) = match &self.config {
            Config::Tcp { ip } => (
                "tcp",
                Some(ip.clone()),
                String::new(),
                String::new(),
                String::new(),
            ),
            Config::WebRtc { ids } => (
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
                config_type,
                tcp_ip,
                web_rtc_own_id,
                web_rtc_remote_id,
                web_rtc_room_id
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(id)
        .bind(self.name.clone())
        .bind(config_type)
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
            "SELECT id, name, config_type, tcp_ip, web_rtc_own_id, web_rtc_remote_id, web_rtc_room_id
             FROM user_connections
             WHERE id = ?1",
        )
            .bind(&id)
            .fetch_optional(conn)
            .await?;

        if let Some(row) = row {
            let config_type: String = row.try_get(2)?;
            let config = match config_type.as_str() {
                "tcp" => Config::Tcp {
                    ip: row.try_get::<Option<String>, _>(3)?.unwrap_or_default(),
                },
                "webrtc" => Config::WebRtc {
                    ids: WebRtcIds {
                        own_id: row.try_get(4)?,
                        remote_id: row.try_get(5)?,
                        room_id: row.try_get(6)?,
                    },
                },
                other => anyhow::bail!("Unknown connection config type '{}'.", other),
            };

            Ok(Some(Self {
                id: row.try_get(0)?,
                name: row.try_get(1)?,
                config,
                handler: None,
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
            "SELECT id, name, config_type, tcp_ip, web_rtc_own_id, web_rtc_remote_id, web_rtc_room_id
             FROM user_connections
             ORDER BY id",
        )
            .fetch_all(conn)
            .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let config_type: String = row.try_get(2)?;
            let config = match config_type.as_str() {
                "tcp" => Config::Tcp {
                    ip: row.try_get::<Option<String>, _>(3)?.unwrap_or_default(),
                },
                "webrtc" => Config::WebRtc {
                    ids: WebRtcIds {
                        own_id: row.try_get(4)?,
                        remote_id: row.try_get(5)?,
                        room_id: row.try_get(6)?,
                    },
                },
                other => anyhow::bail!("Unknown connection config type '{}'.", other),
            };

            items.push(Self {
                id: row.try_get(0)?,
                name: row.try_get(1)?,
                config,
                handler: None,
            });
        }

        Ok(items)
    }
}

#[cfg(any(feature = "desktop", feature = "mobile"))]
#[component]
pub fn ConnectionView(
    connection: Connection,
    on_save_name: EventHandler<(Id, Option<String>)>,
) -> Element {
    let mut name_input = use_signal(|| connection.name.clone().unwrap_or_default());
    let connection_id = connection.id.inner().to_string();

    let config_text = match &connection.config {
        Config::Tcp { ip } => format!("TCP ({ip})"),
        Config::WebRtc { .. } => "WebRTC".to_string(),
    };

    rsx! {
        div { style: "border: 1px solid #ddd; border-radius: 6px; padding: 10px; margin: 8px 0;",

            div { style: "margin-bottom: 6px; font-family: monospace; font-size: 0.9em;",
                "{connection_id}"
            }
            div { style: "margin-bottom: 8px; color: #666;", "{config_text}" }
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
