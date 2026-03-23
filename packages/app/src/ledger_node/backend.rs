use crate::block_entry::{AppBlock, BlockEntry};
use crate::ledger_node::{identification::Identification, ToBackend, ToFrontend};
use crate::money::Money;
use crate::peer_connection;
use crate::peer_connection::{Connection as PeerConnection, WebRtcIds};
use crate::{KeyValue, Value};
use anyhow::Result;
use blockchain::Blockchain;
use core_types::Timestamp;
use db::DbEntity;
use std::path::Path;
use tokio::sync::{mpsc, oneshot};

type LedgerNodeId = blockchain::blockchain::Id;

// communication to peer connection runtimes
pub mod con {

    use super::*;
    use crate::peer_connection::handler::Id;
    #[derive(helper::ServiceWrapper)]
    pub enum Msg {
        Stopped(Id),
        Failed {
            id: Id,
            error: anyhow::Error,
        },
        NewConnectionEstablished {
            orig: u8,
            connection_id: blockchain::Id,
            web_rtc_ids: WebRtcIds,
        },
        NameAcceptRequired {
            connection_id: blockchain::Id,
            private_chain_id: blockchain::Id,
            first_name: String,
            last_name: String,
            middle_name: Option<String>,
            birthday: Option<chrono::NaiveDate>,
        },
    }

    #[derive(Clone)]
    pub struct Service {
        tx: tokio::sync::mpsc::Sender<Msg>,
    }
    impl Service {
        pub fn new(tx: tokio::sync::mpsc::Sender<Msg>) -> Self {
            Self { tx }
        }
    }

    impl PartialEq for Service {
        fn eq(&self, other: &Self) -> bool {
            self.tx.same_channel(&other.tx)
        }
    }

    pub type LedgerNode = Service;
}

/// communication to gui
#[derive(helper::ServiceWrapper)]
pub enum Msg {
    GetPublicPrivate(oneshot::Sender<Result<(blockchain::Id, blockchain::Id)>>),
    GetCurrentMoney(oneshot::Sender<Result<Money>>),
    FromGui(ToBackend, oneshot::Sender<Result<Option<ToFrontend>>>),
}

/// Events pushed proactively from LedgerNode to the GUI.
#[derive(Clone, PartialEq)]
pub enum AsyncEvent {
    FrontendEvent(ToFrontend),
    /// A peer introduced itself and is waiting for the user to accept or reject.
    WaitForNameAccept {
        connection_id: blockchain::Id,
        private_chain_id: blockchain::Id,
        first_name: String,
        last_name: String,
        middle_name: Option<String>,
        birthday: Option<chrono::NaiveDate>,
    },
    /// A peer connection was fully established and persisted.
    ConnectionEstablished {
        new_connection_id: u8,
        connection: peer_connection::Connection,
    },
    ConnectionEstablishedFailed {
        new_connection_id: u8,
    },
    /// An active peer connection was lost (stopped or failed).
    ConnectionLost {
        id: blockchain::Id,
    },
}

#[derive(Clone)]
pub struct Service {
    tx: tokio::sync::mpsc::Sender<Msg>,
}

impl PartialEq for Service {
    fn eq(&self, other: &Self) -> bool {
        self.tx.same_channel(&other.tx)
    }
}

impl Service {
    pub fn empty() -> Self {
        let (tx, _rx) = tokio::sync::mpsc::channel(32);
        Self { tx }
    }

    pub async fn create_new_instance(
        base_path: impl AsRef<Path>,
        id: &LedgerNodeId,
        identification: Identification,
    ) -> Result<(Self, mpsc::Receiver<AsyncEvent>)> {
        let (mut internal, event_rx) =
            Internal::create_new_instance(base_path, id, identification).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            internal.process(rx).await;
        });
        Ok((Self { tx }, event_rx))
    }

    pub async fn open(
        base_path: impl AsRef<Path>,
        id: &LedgerNodeId,
    ) -> Result<(Self, mpsc::Receiver<AsyncEvent>)> {
        let (mut internal, event_rx) = Internal::open(base_path, id).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            internal.process(rx).await;
        });
        Ok((Self { tx }, event_rx))
    }
}

struct Internal {
    private_chain_db: db::DB,
    key_db: db::DB,
    public_chain_db: db::DB,
    private_chain: Blockchain<BlockEntry>,
    public_chain: Blockchain<BlockEntry>,
    user_connections: Vec<PeerConnection>,
    new_connections: Vec<(u8, peer_connection::Service)>,
    init_id: u8,
    connection_tx: con::Service,
    connection_events_rx: mpsc::Receiver<con::Msg>,
    event_tx: mpsc::Sender<AsyncEvent>,
    current: Money,
    new_connection_web_rtc_ids: WebRtcIds,
}

impl Internal {
    const CURRENT_AMOUNT_MAX_AGE_SECS: u64 = 24 * 60 * 60;

    /// Opens an existing LedgerNode.
    pub async fn open(
        base_path: impl AsRef<Path>,
        id: &LedgerNodeId,
    ) -> Result<(Self, mpsc::Receiver<AsyncEvent>)> {
        Self::open_internal(base_path, id).await
    }

    /// Should be called on first creation of a LedgerNode to initialize the databases and first blocks.
    pub async fn create_new_instance(
        base_path: impl AsRef<Path>,
        id: &LedgerNodeId,
        identification: Identification,
    ) -> Result<(Self, mpsc::Receiver<AsyncEvent>)> {
        let (mut ret, event_rx) = Self::open_internal(base_path, id).await?;

        // Store the first identification in key_db.
        KeyValue::new_typed(
            Identification::storage_key(0),
            Value::Object(identification.to_json()?),
        )
        .write(ret.key_db.connection())
        .await?;
        KeyValue::new_typed(Identification::count_key().to_string(), 1i64)
            .write(ret.key_db.connection())
            .await?;

        let private_pub = ret.add_key_pair(ret.private_chain.id()).await?;
        let fist_private = BlockEntry::new_verification(&private_pub)?;
        let first_block = ret
            .private_chain
            .append_entries_with_db(vec![fist_private], &ret.private_chain_db)
            .await?;

        let public_pub = ret.add_key_pair(ret.public_chain.id()).await?;
        let fist_public = BlockEntry::new_verification(&public_pub)?;
        let first_link = blockchain::BlockLink::from_block(ret.private_chain.id(), &first_block)?;
        // Record the identification hash on the public chain.
        let ident_hash = BlockEntry::Identification {
            data: identification.hash_bytes(),
        };

        ret.public_chain
            .append_entries_with_db(
                vec![fist_public, BlockEntry::from_link(first_link)?, ident_hash],
                &ret.public_chain_db,
            )
            .await?;
        Ok((ret, event_rx))
    }

    async fn open_internal<P: AsRef<Path>>(
        base_path: P,
        id: &LedgerNodeId,
    ) -> Result<(Self, mpsc::Receiver<AsyncEvent>)> {
        let base_path = base_path.as_ref();
        let user_dir = base_path.join(id.inner().to_string());

        std::fs::create_dir_all(&user_dir)?;

        let private_chain_db_path = user_dir.join("private.sqlite");
        let public_chain_db_path = user_dir.join("public.sqlite");
        let key_db_path = user_dir.join("key.sqlite");

        let mut private_chain_db = db::DB::open(&private_chain_db_path).await?;
        let mut public_chain_db = db::DB::open(&public_chain_db_path).await?;
        let mut key_db = db::DB::open(&key_db_path).await?;

        Self::init_private_db(&mut private_chain_db).await?;
        Self::init_public_db(&mut public_chain_db).await?;
        Self::init_key_db(&mut key_db).await?;

        let private_chain = Blockchain::<BlockEntry>::from_db(&mut private_chain_db).await?;
        let public_chain = Blockchain::<BlockEntry>::from_db(&mut public_chain_db).await?;
        let user_connections = PeerConnection::list(key_db.connection()).await?;
        let new_connection_web_rtc_ids = WebRtcIds::new();
        let (connection_events_tx, connection_events_rx) = mpsc::channel(128);
        let (event_tx, event_rx) = mpsc::channel(64);

        let mut ret = Self {
            private_chain_db,
            public_chain_db,
            key_db,
            private_chain,
            public_chain,
            user_connections: user_connections,
            new_connections: vec![],
            connection_tx: con::Service::new(connection_events_tx),
            init_id: 0,
            connection_events_rx,
            event_tx,
            current: Money::default(),
            new_connection_web_rtc_ids,
        };

        ret.ensure_recent_current_amount_snapshot().await?;
        ret.current = ret.last_current_amount().unwrap_or_default();

        Ok((ret, event_rx))
    }

    /// Initializes the private database (contains user info, keys, blockchains)
    async fn init_private_db(private_db: &mut db::DB) -> Result<()> {
        // User table via User DbEntity
        private_db.migrate_table::<AppBlock>().await?;

        Ok(())
    }
    async fn init_key_db(private_db: &mut db::DB) -> Result<()> {
        // User table via User DbEntity
        private_db.migrate_table::<crypto::KeyMeta>().await?;
        private_db.migrate_table::<KeyValue>().await?;
        private_db.migrate_table::<PeerConnection>().await?;
        private_db
            .migrate_table::<peer_connection::TrustState>()
            .await?;

        Ok(())
    }

    /// Initializes the public database (only blockchains)
    async fn init_public_db(public_db: &mut db::DB) -> Result<()> {
        // Blockchains table via Blockchain DbEntity
        public_db.migrate_table::<AppBlock>().await?;
        Ok(())
    }

    async fn add_key_pair<T>(&self, id: &helper::UId<T>) -> Result<crypto::KeyMeta> {
        crypto::KeyMeta::create_ed25519(id, self.key_db.connection()).await
    }

    /// Loads all identifications stored in key_db.
    async fn load_identifications(&self) -> Result<Vec<Identification>> {
        let count_kv =
            KeyValue::read(self.key_db.connection(), &Identification::count_key().to_string())
                .await?;
        let count: u32 = count_kv
            .and_then(|kv| kv.get_value::<i64>().ok())
            .map(|v| v as u32)
            .unwrap_or(0);

        let mut identifications = Vec::with_capacity(count as usize);
        for i in 0..count {
            let key = Identification::storage_key(i);
            if let Some(kv) = KeyValue::read(self.key_db.connection(), &key).await? {
                if let Ok(json) = kv.get_value::<String>() {
                    if let Ok(ident) = Identification::from_json(&json) {
                        identifications.push(ident);
                    }
                }
            }
        }
        Ok(identifications)
    }

    /// Stores a new identification in key_db and records its hash on the public chain.
    async fn add_identification(&mut self, identification: Identification) -> Result<()> {
        let count_kv =
            KeyValue::read(self.key_db.connection(), &Identification::count_key().to_string())
                .await?;
        let count: u32 = count_kv
            .and_then(|kv| kv.get_value::<i64>().ok())
            .map(|v| v as u32)
            .unwrap_or(0);

        KeyValue::new_typed(
            Identification::storage_key(count),
            Value::Object(identification.to_json()?),
        )
        .write(self.key_db.connection())
        .await?;

        KeyValue::new_typed(Identification::count_key().to_string(), (count + 1) as i64)
            .write(self.key_db.connection())
            .await?;

        self.public_chain
            .append_entries_with_db(
                vec![BlockEntry::Identification {
                    data: identification.hash_bytes(),
                }],
                &self.public_chain_db,
            )
            .await?;

        Ok(())
    }

    fn last_current_amount(&self) -> Option<Money> {
        self.private_chain.last_entry_map(|entry| match entry {
            BlockEntry::CurrentAmount { money } => Some(*money),
            _ => None,
        })
    }

    async fn ensure_recent_current_amount_snapshot(&mut self) -> Result<()> {
        let now = Timestamp::now();

        let next_money = match self.last_current_amount() {
            Some(last_money) => {
                let age = now - last_money.timestamp();
                if age <= Self::CURRENT_AMOUNT_MAX_AGE_SECS {
                    return Ok(());
                }
                last_money.on_time(now)
            }
            None => Money::default(),
        };

        self.private_chain
            .append_entries_with_db(
                vec![BlockEntry::CurrentAmount { money: next_money }],
                &self.private_chain_db,
            )
            .await?;

        Ok(())
    }

    async fn on_connection_event(&mut self, event: con::Msg) {
        match event {
            con::Msg::NewConnectionEstablished {
                orig,
                connection_id,
                web_rtc_ids,
            } => {
                log::info!("New connection established: {}", connection_id);

                if let Some(index) = self
                    .new_connections
                    .iter()
                    .position(|connection| connection.0 == orig)
                {
                    let (_, new_connection) = self.new_connections.swap_remove(index);
                    let connection =
                        PeerConnection::from_service(connection_id, web_rtc_ids, new_connection);

                    let write_result = connection.write(self.key_db.connection()).await;
                    let result = match write_result {
                        Ok(()) => {
                            let conn = connection.clone();
                            self.user_connections.push(conn);
                            Ok(())
                        }
                        Err(error) => Err(error),
                    };
                    let _ = self
                        .event_tx
                        .send(AsyncEvent::ConnectionEstablished {
                            new_connection_id: orig,
                            connection: connection.clone(),
                        })
                        .await;
                }
            }

            con::Msg::Stopped(from) => match from {
                crate::peer_connection::handler::Id::Established(id) => {
                    for itr in &mut self.user_connections {
                        if *itr.get_id() == id {
                            itr.stopped();
                        }
                    }
                    let _ = self
                        .event_tx
                        .send(AsyncEvent::ConnectionLost { id: id })
                        .await;
                }
                crate::peer_connection::handler::Id::Creating(id) => {
                    self.new_connections.retain(|conn| conn.0 != id);
                }
            },
            con::Msg::Failed { id, error } => match id {
                crate::peer_connection::handler::Id::Established(id) => {
                    for itr in &mut self.user_connections {
                        if *itr.get_id() == id {
                            itr.stopped();
                        }
                    }
                    let _ = self
                        .event_tx
                        .send(AsyncEvent::ConnectionLost { id: id })
                        .await;
                }
                crate::peer_connection::handler::Id::Creating(id) => {
                    if let Some(index) = self
                        .new_connections
                        .iter()
                        .position(|connection| connection.0 == id)
                    {
                        let (_, _) = self.new_connections.swap_remove(index);
                        let _ = self
                            .event_tx
                            .send(AsyncEvent::ConnectionEstablishedFailed {
                                new_connection_id: id,
                            })
                            .await;
                    }
                }
            },
            con::Msg::NameAcceptRequired {
                connection_id,
                private_chain_id,
                first_name,
                last_name,
                middle_name,
                birthday,
            } => {
                /*    let _ = self
                .event_tx
                .send(ToFrontend::WaitForNameAccept {
                    connection_id,
                    private_chain_id,
                    first_name,
                    last_name,
                    middle_name,
                    birthday,
                })
                .await;*/
            }
        }
    }
}

impl Internal {
    pub async fn process(&mut self, mut ui_rx: tokio::sync::mpsc::Receiver<Msg>) {
        loop {
            tokio::select! {
                Some(msg) = ui_rx.recv() => {
                    match msg{
                        Msg::GetPublicPrivate(resp) => {
                            let result = Ok((self.private_chain.id().clone(), self.public_chain.id().clone()));
                            let _ = resp.send(result);
                        }
                        Msg::GetCurrentMoney(tx) => {
                            let _ = tx.send(Ok(self.current));
                        }
                        Msg::FromGui(msg, resp) => {
                            let result = self.handle_ui_msg(msg).await;
                            let _ = resp.send(result);
                        }
                    }
                }
                Some(event) = self.connection_events_rx.recv() => {
                    self.on_connection_event(event).await;
                }
                else => {
                    break;
                }
            }
        }
    }

    async fn handle_ui_msg(&mut self, msg: ToBackend) -> Result<Option<ToFrontend>> {
        match msg {
            ToBackend::Init => {
                let identifications = self.load_identifications().await?;
                Ok(Some(ToFrontend::Initialized {
                    private: self.private_chain.id().clone(),
                    public: self.public_chain.id().clone(),
                    money: self.current,
                    identifications,
                }))
            }
            ToBackend::GetBlocks {
                id,
                count,
                start_at,
            } => {
                if self.private_chain.id() == &id {
                    let blocks = self.private_chain.block_from(count, start_at).await;
                    Ok(Some(ToFrontend::Blocks { id, blocks }))
                } else if self.public_chain.id() == &id {
                    let blocks = self.public_chain.block_from(count, start_at).await;
                    Ok(Some(ToFrontend::Blocks { id, blocks }))
                } else {
                    Err(anyhow::anyhow!("Blockchain not found"))
                }
            }
            ToBackend::GetIdentifications => {
                let identifications = self.load_identifications().await?;
                Ok(Some(ToFrontend::Identifications(identifications)))
            }
            ToBackend::AddIdentification(identification) => {
                self.add_identification(identification).await?;
                let identifications = self.load_identifications().await?;
                Ok(Some(ToFrontend::Identifications(identifications)))
            }
            ToBackend::StartNewConnection { new_connection_id } => {
                self.new_connection_web_rtc_ids = WebRtcIds::new();
                let ids = self.new_connection_web_rtc_ids.clone();

                self.new_connections.push((
                    new_connection_id,
                    peer_connection::Service::start_init(
                        new_connection_id,
                        ids.clone(),
                        self.connection_tx.clone(),
                    ),
                ));

                Ok(Some(ToFrontend::ConnectionsIds {
                    new_connection_id: self.init_id,
                    ids: ids.clone(),
                }))
            }
            ToBackend::CreateNewConnectionFromWebRtcIds {
                new_connection_id,
                ids,
            } => {
                self.new_connections.push((
                    new_connection_id,
                    peer_connection::Service::start_init(
                        new_connection_id,
                        ids,
                        self.connection_tx.clone(),
                    ),
                ));
                Ok(None)
            }

            ToBackend::GetPeerConnections => Ok(Some(ToFrontend::PeerConnections {
                connections: self.user_connections.clone(),
            })),
            ToBackend::UpdatePeerConnectionName { id, name } => {
                match self
                    .user_connections
                    .iter_mut()
                    .find(|connection| connection.id == id)
                {
                    Some(connection) => {
                        connection.name = name;
                        match connection.write(self.key_db.connection()).await {
                            Ok(()) => Ok(None),
                            Err(error) => Err(error),
                        }
                    }
                    None => Err(anyhow::anyhow!("Peer connection not found")),
                }
            }
        }
    }
}
