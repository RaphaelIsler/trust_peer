use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ledger_node::backend::{AsyncEvent, Service as LedgerService};
use crate::{ToBackend, ToFrontend};
use anyhow::Result;
use tokio::sync::mpsc;

pub struct Service {
    rx_backend: mpsc::Receiver<ToBackend>,
    tx_frontend: mpsc::Sender<ToFrontend>,
    ledger_nodes: HashMap<String, LedgerService>,
    ledger_events_tx: mpsc::Sender<(blockchain::blockchain::Id, AsyncEvent)>,
    ledger_events: mpsc::Receiver<(blockchain::blockchain::Id, AsyncEvent)>,
    base_path: PathBuf,
}

impl Service {
    pub fn start(
        base_path: impl AsRef<Path>,
        rx_backend: mpsc::Receiver<ToBackend>,
        tx_frontend: mpsc::Sender<ToFrontend>,
    ) {
        let path = base_path.as_ref().to_path_buf();
        let (ledger_events_tx, ledger_events) = mpsc::channel(64);
        tokio::spawn(async move {
            let mut service = Self {
                rx_backend,
                tx_frontend,
                ledger_nodes: HashMap::new(),
                ledger_events_tx,
                ledger_events,
                base_path: path,
            };
            if let Err(e) = service.init().await {
                log::error!("Backend init failed: {e}");
            }
            service.run().await;
        });
    }

    /// Discovers existing LedgerNode directories under base_path.
    /// A valid directory has a UUID name and contains `private.sqlite`.
    fn discover_ledger_nodes(base_path: &Path) -> Vec<blockchain::blockchain::Id> {
        let mut ids = Vec::new();
        if !base_path.is_dir() {
            return ids;
        }
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.join("private.sqlite").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if let Ok(id) = blockchain::blockchain::Id::parse_str(name) {
                            ids.push(id);
                        }
                    }
                }
            }
        }
        ids
    }

    async fn init(&mut self) -> Result<()> {
        let ids = Self::discover_ledger_nodes(&self.base_path);
        for id in ids {
            match LedgerService::open(self.base_path.clone(), &id).await {
                Ok((service, event_rx)) => {
                    let (_public, private) = service.get_public_private().await?;
                    self.spawn_event_forwarder(private.clone(), event_rx);
                    self.ledger_nodes.insert(private.to_string(), service);
                }
                Err(e) => {
                    log::warn!("Failed to open ledger node {}: {}", id.to_string(), e);
                }
            }
        }
        Ok(())
    }

    fn spawn_event_forwarder(
        &self,
        private: blockchain::blockchain::Id,
        mut event_rx: mpsc::Receiver<AsyncEvent>,
    ) {
        let tx = self.ledger_events_tx.clone();
        tokio::spawn(async move {
            while let Some(event) = event_rx.recv().await {
                if tx.send((private.clone(), event)).await.is_err() {
                    break;
                }
            }
        });
    }

    fn async_event_to_frontend(event: AsyncEvent) -> Option<crate::ledger_node::ToFrontend> {
        use crate::ledger_node::ToFrontend as LedgerMsg;
        match event {
            AsyncEvent::FrontendEvent(msg) => Some(msg),
            AsyncEvent::WaitForNameAccept {
                connection_id,
                private_chain_id: _,
                first_name,
                last_name,
                middle_name,
                birthday,
            } => Some(LedgerMsg::WaitForNameAccept {
                connection_id,
                identification: crate::ledger_node::Identification::NameAndBirth {
                    first_name,
                    last_name,
                    middle_name,
                    birth_date: birthday.unwrap_or_default(),
                },
            }),
            AsyncEvent::ConnectionEstablished {
                new_connection_id,
                connection,
            } => Some(LedgerMsg::ConnectionEstablished {
                new_connection_id,
                connection: connection.id,
            }),
            AsyncEvent::ConnectionEstablishedFailed { new_connection_id } => {
                Some(LedgerMsg::ConnectionEstablishedFailed { new_connection_id })
            }
            AsyncEvent::ConnectionLost { id } => Some(LedgerMsg::ConnectionLost { connection: id }),
        }
    }

    async fn build_init_entries(&self) -> Result<Vec<(blockchain::blockchain::Id, crate::Money)>> {
        let mut entries = Vec::new();
        for service in self.ledger_nodes.values() {
            let money = service.get_current_money().await?;
            let (_public, private) = service.get_public_private().await?;
            entries.push((private, money));
        }
        Ok(entries)
    }

    async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(message) = self.rx_backend.recv() => {
                    match self.handle_frontend(message).await {
                        Ok(Some(response)) => {
                            let _ = self.tx_frontend.send(response).await;
                        }
                        Ok(None) => {}
                        Err(e) => log::error!("Backend error: {e}"),
                    }
                },
                Some((private, event)) = self.ledger_events.recv() => {
                    if let Some(msg) = Self::async_event_to_frontend(event) {
                        let _ = self.tx_frontend.send(ToFrontend::Ledger { private, msg }).await;
                    }
                },
            }
        }
    }

    async fn handle_frontend(&mut self, message: ToBackend) -> Result<Option<ToFrontend>> {
        Ok(match message {
            ToBackend::Init => Some(ToFrontend::Init(self.build_init_entries().await?)),
            ToBackend::Create(identification) => {
                let new_id = blockchain::blockchain::Id::new();
                let (service, event_rx) = LedgerService::create_new_instance(
                    &self.base_path,
                    &new_id,
                    identification,
                )
                .await?;
                let (_public, private) = service.get_public_private().await?;
                self.spawn_event_forwarder(private.clone(), event_rx);
                self.ledger_nodes.insert(private.to_string(), service);
                Some(ToFrontend::Init(self.build_init_entries().await?))
            }
            ToBackend::Ledger { private, msg } => {
                if let Some(ledger) = self.ledger_nodes.get_mut(&private.to_string()) {
                    if let Some(back) = ledger.from_gui(msg).await? {
                        Some(ToFrontend::Ledger { private, msg: back })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        })
    }
}
