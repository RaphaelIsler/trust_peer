use crate::block_entry::{AppBlock, BlockEntry};
use crate::money::Money;
use crate::peer_connection;
use crate::peer_connection::{Connection as PeerConnection, WebRtcIds};
use crate::{KeyValue, User, UserId};
use anyhow::Result;
use blockchain::Blockchain;
use chrono::NaiveDate;
use core_types::Timestamp;
use db::DbEntity;
use std::path::Path;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

/// communication to gui
pub mod ui {
    use super::*;

    #[derive(helper::ServiceWrapper)]
    pub enum Msg {
        GetPrivateAndPublic(
            oneshot::Sender<Result<(blockchain::blockchain::Id, blockchain::blockchain::Id)>>,
        ),
        GetBlocks {
            id: blockchain::blockchain::Id,
            count: usize,
            start_at: Option<usize>,
            tx: oneshot::Sender<Result<Vec<AppBlock>>>,
        },
        StartNewConnection {
            ids_tx: oneshot::Sender<Result<WebRtcIds>>,
            done: oneshot::Sender<Result<()>>,
        },
        CreateNewConnectionFromWebRtcIds {
            ids: WebRtcIds,
            tx: oneshot::Sender<Result<()>>,
        },
        GetPeerConnections(oneshot::Sender<Result<Vec<PeerConnection>>>),
        UpdatePeerConnectionName {
            id: blockchain::blockchain::Id,
            name: Option<String>,
            tx: oneshot::Sender<Result<()>>,
        },
    }

    /// Events pushed proactively from LedgerNode to the GUI.
    #[derive(Clone, PartialEq)]
    pub enum LedgerEvent {
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
            connection: peer_connection::Connection,
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
            user: &User,
            middle_name: Option<String>,
            date_of_birth: NaiveDate,
        ) -> Result<(Self, mpsc::Receiver<LedgerEvent>)> {
            let (mut internal, event_rx) =
                Internal::create_new_instance(base_path, user, middle_name, date_of_birth).await?;
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            tokio::spawn(async move {
                internal.process(rx).await;
            });
            Ok((Self { tx }, event_rx))
        }

        pub async fn start(base_path: impl AsRef<Path>, user_id: &UserId) -> Result<(Self, mpsc::Receiver<LedgerEvent>)> {
            let (mut internal, event_rx) = Internal::open(base_path, user_id).await?;
            let (tx, rx) = tokio::sync::mpsc::channel(32);
            tokio::spawn(async move {
                internal.process(rx).await;
            });
            Ok((Self { tx }, event_rx))
        }
    }

    pub type LedgerNode = Service;
}

// communication to peer connection runtimes
pub mod con {
    use std::any;

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

struct Internal {
    private_chain_db: db::DB,
    key_db: db::DB,
    public_chain_db: db::DB,
    private_chain: Blockchain<BlockEntry>,
    public_chain: Blockchain<BlockEntry>,
    user_connections: Vec<PeerConnection>,
    new_connections: Vec<(
        u8,
        peer_connection::Service,
        tokio::sync::oneshot::Sender<Result<()>>,
    )>,
    init_id: u8,
    connection_tx: con::Service,
    connection_events_rx: mpsc::Receiver<con::Msg>,
    event_tx: mpsc::Sender<ui::LedgerEvent>,
    new_connection_web_rtc_ids: WebRtcIds,
}

impl Internal {
    const CURRENT_AMOUNT_MAX_AGE_SECS: u64 = 24 * 60 * 60;

    /// Opens the ledger node with a message receiver (used for per-user workers)
    pub async fn open(base_path: impl AsRef<Path>, user_id: &UserId) -> Result<(Self, mpsc::Receiver<ui::LedgerEvent>)> {
        Self::open_internal(base_path, user_id).await
    }

    /// Should be call on first creation of a user to initialize the databases and create the first blocks
    pub async fn create_new_instance(
        base_path: impl AsRef<Path>,
        user: &User,
        middle_name: Option<String>,
        date_of_birth: NaiveDate,
    ) -> Result<(Self, mpsc::Receiver<ui::LedgerEvent>)> {
        let user_id = user.id.clone();

        let (mut ret, event_rx) = Self::open_internal(base_path, &user_id).await?;

        if let Some(middle_name) = middle_name {
            KeyValue::new_typed("user.middle_name".to_string(), middle_name)
                .write(ret.key_db.connection())
                .await?;
        }

        KeyValue::new_typed("user.date_of_birth".to_string(), date_of_birth.to_string())
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

        ret.public_chain
            .append_entries_with_db(
                vec![fist_public, BlockEntry::from_link(first_link)?],
                &ret.public_chain_db,
            )
            .await?;
        Ok((ret, event_rx))
    }

    async fn open_internal<P: AsRef<Path>>(base_path: P, user_id: &UserId) -> Result<(Self, mpsc::Receiver<ui::LedgerEvent>)> {
        let base_path = base_path.as_ref();
        let user_dir = base_path.join(user_id.inner().to_string());

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
            new_connection_web_rtc_ids,
        };

        ret.ensure_recent_current_amount_snapshot().await?;

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
        private_db.migrate_table::<peer_connection::State>().await?;

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
                    let (_, new_connection, done) = self.new_connections.swap_remove(index);
                    let connection = PeerConnection::from_service(connection_id, web_rtc_ids, new_connection);

                    let write_result = connection.write(self.key_db.connection()).await;
                    let result = match write_result {
                        Ok(()) => {
                            let conn = connection.clone();
                            self.user_connections.push(connection);
                            let _ = self
                                .event_tx
                                .send(ui::LedgerEvent::ConnectionEstablished { connection: conn })
                                .await;
                            Ok(())
                        }
                        Err(error) => Err(error),
                    };
                    done.send(result).ok();
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
                        .send(ui::LedgerEvent::ConnectionLost { id })
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
                        .send(ui::LedgerEvent::ConnectionLost { id })
                        .await;
                }
                crate::peer_connection::handler::Id::Creating(id) => {
                    if let Some(index) = self
                        .new_connections
                        .iter()
                        .position(|connection| connection.0 == id)
                    {
                        let (_, _, done) = self.new_connections.swap_remove(index);
                        let _ = done.send(Err(error));
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
                let _ = self
                    .event_tx
                    .send(ui::LedgerEvent::WaitForNameAccept {
                        connection_id,
                        private_chain_id,
                        first_name,
                        last_name,
                        middle_name,
                        birthday,
                    })
                    .await;
            }
        }
    }
}

impl Internal {
    pub async fn process(&mut self, mut ui_rx: tokio::sync::mpsc::Receiver<ui::Msg>) {
        loop {
            tokio::select! {
                Some(msg) = ui_rx.recv() => {
                    self.handle_ui_msg(msg).await;
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

    async fn handle_ui_msg(&mut self, msg: ui::Msg) {
        use ui::Msg;
        match msg {
            Msg::GetBlocks {
                id,
                count,
                start_at,
                tx,
            } => {
                if self.private_chain.id() == &id {
                    let blocks = self.private_chain.block_from(count, start_at).await;
                    let _ = tx.send(Ok(blocks));
                } else if self.public_chain.id() == &id {
                    let blocks = self.public_chain.block_from(count, start_at).await;
                    let _ = tx.send(Ok(blocks));
                } else {
                    let _ = tx.send(Err(anyhow::anyhow!("Blockchain not found")));
                }
            }
            Msg::GetPrivateAndPublic(tx) => {
                let _ = tx.send(Ok((
                    self.private_chain.id().clone(),
                    self.public_chain.id().clone(),
                )));
            }
            Msg::StartNewConnection { ids_tx, done } => {
                self.new_connection_web_rtc_ids = WebRtcIds::new();
                let ids = self.new_connection_web_rtc_ids.clone();

                let _ = ids_tx.send(Ok(ids.clone()));
                self.init_id += 1;
                if self.init_id > 100 {
                    self.init_id = 0;
                }

                self.new_connections.push((
                    self.init_id,
                    peer_connection::Service::start_init(
                        self.init_id,
                        ids,
                        self.connection_tx.clone(),
                    ),
                    done,
                ));
            }
            Msg::CreateNewConnectionFromWebRtcIds { ids, tx } => {
                self.init_id += 1;
                if self.init_id > 100 {
                    self.init_id = 0;
                }
                self.new_connections.push((
                    self.init_id,
                    peer_connection::Service::start_init(
                        self.init_id,
                        ids,
                        self.connection_tx.clone(),
                    ),
                    tx,
                ));
            }

            Msg::GetPeerConnections(tx) => {
                let _ = tx.send(Ok(self.user_connections.clone()));
            }
            Msg::UpdatePeerConnectionName { id, name, tx } => {
                match self
                    .user_connections
                    .iter_mut()
                    .find(|connection| connection.id == id)
                {
                    Some(connection) => {
                        connection.name = name;
                        match connection.write(self.key_db.connection()).await {
                            Ok(()) => {
                                let _ = tx.send(Ok(()));
                            }
                            Err(error) => {
                                let _ = tx.send(Err(error));
                            }
                        }
                    }
                    None => {
                        let _ = tx.send(Err(anyhow::anyhow!("Peer connection not found")));
                    }
                }
            }
        }
    }
}
