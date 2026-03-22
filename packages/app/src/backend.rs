use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ledger_node::backend::{AsyncEvent, Service as LedgerService};
use crate::{ToBackend, ToFrontend};
use anyhow::Result;
use tokio::sync::mpsc;

pub struct Service {
    rx_backend: mpsc::Receiver<ToBackend>,
    tx_frontend: mpsc::Sender<ToFrontend>,
    user: crate::user::backend::Service,
    ledger_nodes: HashMap<String, (crate::user::User, LedgerService)>,
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
                user: crate::user::backend::Service::new(),
                ledger_nodes: HashMap::new(),
                ledger_events_tx,
                ledger_events,
                base_path: path,
            };
            service.init().await;
            service.run().await;
        });
    }
    async fn init(&mut self) -> Result<()> {
        self.user.init().await?;
        let users = self.user.load_users().await?;
        for user in &users.users {
            let (service, event_rx) = LedgerService::start(self.base_path.clone(), user.id()).await?;
            let (_public, private) = service.get_public_private().await?;
            self.spawn_event_forwarder(private.clone(), event_rx);
            self.ledger_nodes.insert(private.to_string(), (user.clone(), service));
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
                identification: crate::user::Identification::NameAndBirth {
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
            AsyncEvent::ConnectionLost { id } => {
                Some(LedgerMsg::ConnectionLost { connection: id })
            }
        }
    }

    async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(message) = self.rx_backend.recv() => {
                    if let Some(response) = self.handle_frontend(message).await.unwrap() {
                        self.tx_frontend.send(response).await.unwrap();
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
            ToBackend::Init => {
                let mut entries = Vec::new();
                for (private_str, (user, service)) in &self.ledger_nodes {
                    let money = service.get_current_money().await?;
                    let (_, private) = service.get_public_private().await?;
                    entries.push((user.clone(), private, money));
                }
                Some(ToFrontend::Init(entries))
            }
            ToBackend::User(user) => {
                if let Some(resp) = self.user.handle_frontend(user).await? {
                    Some(ToFrontend::User(resp))
                } else {
                    None
                }
            }
            ToBackend::Ledger { private, msg } => {
                if let Some((_, ledger)) = self.ledger_nodes.get_mut(&private.to_string()) {
                    if let Some(back) = ledger.from_gui(msg).await? {
                        Some(ToFrontend::Ledger {
                            private: private,
                            msg: back,
                        })
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
