use anyhow::Result;
use chrono::NaiveDate;
use blockchain::Blockchain;
use crate::block_entry::{AppBlock, BlockEntry};
use crate::money::Money;
use crate::timestamp::Timestamp;
use db::DbEntity;
use std::path::Path;
use tokio::sync::oneshot;
use crate::{KeyValue, User, UserConnection, UserConnectionTransport, UserId, WebRtcIds};
use p2p_webrtc::{P2pConfig, P2pWebRtc};
use p2p_webrtc::data_channel::DataChannel;

#[derive(helper::ServiceWrapper)]
pub enum Msg {
    GetPrivateAndPublic(oneshot::Sender<Result<(blockchain::blockchain::Id, blockchain::blockchain::Id)>>),
    GetBlocks{id: blockchain::blockchain::Id, count: usize, start_at: Option<usize>, tx: oneshot::Sender<Result<Vec<AppBlock>>>},
    StartNewConnection{ids_tx: oneshot::Sender<Result<WebRtcIds>>, done: oneshot::Sender<Result<()>>},
    CreateNewConnectionFromWebRtcIds{ids: WebRtcIds, tx: oneshot::Sender<Result<()>>},
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

impl Service{
    pub fn empty() -> Self {
        let (tx, _rx) = tokio::sync::mpsc::channel(32);
        Self { tx }
    }

    pub async fn create_new_instance(
        base_path: impl AsRef<Path>,
        user: &User,
        middle_name: Option<String>,
        date_of_birth: NaiveDate) -> Result<Self> {

        let mut internal = Internal::create_new_instance(base_path, user, middle_name, date_of_birth).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            internal.process(rx).await;
        });
        Ok(Self { tx })
    }

    pub async fn start(base_path: impl AsRef<Path>, user_id: &UserId) -> Result<Self> {
        let mut internal = Internal::open(base_path, user_id).await?;
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        tokio::spawn(async move {
            internal.process(rx).await;
        });
        Ok(Self { tx })

}}

struct Internal{
    private_chain_db: db::DB,
    key_db: db::DB,
    public_chain_db: db::DB,
    private_chain: Blockchain<BlockEntry>,
    public_chain: Blockchain<BlockEntry>,
    #[allow(dead_code)]
    user_connections: Vec<UserConnection>,
    #[allow(dead_code)]
    new_connection_web_rtc_ids: WebRtcIds,
}


impl Internal {
    const CURRENT_AMOUNT_MAX_AGE_SECS: u64 = 24 * 60 * 60;
    const DEFAULT_SIGNALING_SERVER: &'static str = "ws://127.0.0.1:3000";

    /// Opens the user service with a message receiver (used for per-user workers)
    pub async fn open(
        base_path: impl AsRef<Path>,
        user_id: &UserId,
    ) -> Result<Self> {
        Self::open_internal(base_path, user_id).await
    }

    /// Should be call on first creation of a user to initialize the databases and create the first blocks
    pub async fn create_new_instance(
        base_path: impl AsRef<Path>,
        user: &User,
        middle_name: Option<String>,
        date_of_birth: NaiveDate,
    ) -> Result<Self> {
        let user_id = user.id.clone();

        let mut ret = Self::open_internal(base_path, &user_id).await?;
        //user.write(ret.key_db.connection()).await?;

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
        let first_block = ret.private_chain.append_entries_with_db(vec![fist_private], &ret.private_chain_db).await?;

        let public_pub = ret.add_key_pair(ret.public_chain.id()).await?;
        let fist_public = BlockEntry::new_verification(&public_pub)?;
        let first_link = blockchain::BlockLink::from_block(ret.private_chain.id(), &first_block)?;


        ret.public_chain.append_entries_with_db(vec![fist_public, BlockEntry::from_link(first_link)?], &ret.public_chain_db).await?;
        Ok(ret)
    }

    async fn open_internal<P: AsRef<Path>>(
        base_path: P,
        user_id: &UserId,
    ) -> Result<Self> {
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
        let user_connections = UserConnection::list(key_db.connection()).await?;
        let new_connection_web_rtc_ids = WebRtcIds::new();

        let mut ret = Self {
            private_chain_db,
            public_chain_db,
            key_db,
            private_chain,
            public_chain,
            user_connections,
            new_connection_web_rtc_ids,
        };

        ret.ensure_recent_current_amount_snapshot().await?;

        Ok(ret)
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
        private_db.migrate_table::<UserConnection>().await?;

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

    async fn connect_web_rtc_data_channel(
        &self,
        own_peer_id: p2p_webrtc::PeerId,
        room_id: p2p_webrtc::RoomId,
    ) -> Result<DataChannel> {
        let config = P2pConfig::new(Self::DEFAULT_SIGNALING_SERVER.to_string(), room_id)
            .with_peer_id(own_peer_id)
            .with_timeout(30);

        let mut p2p = P2pWebRtc::new(config);
        p2p.connect().await?;

        p2p.data_channel()
            .ok_or_else(|| anyhow::anyhow!("WebRTC connected without data channel"))
    }
}

impl Internal{
    pub async fn process(&mut self, mut rx: tokio::sync::mpsc::Receiver<Msg>) {
        while let Some(msg) = rx.recv().await {
            match msg {
                Msg::GetBlocks{id, count, start_at, tx} => {
                    if self.private_chain.id() == &id{
                        let blocks = self.private_chain.block_from(count, start_at).await;
                        let _ = tx.send(Ok(blocks));
                    } else if self.public_chain.id() == &id{
                        let blocks = self.public_chain.block_from(count, start_at).await;
                        let _ = tx.send(Ok(blocks));
                    } else {
                        let _ = tx.send(Err(anyhow::anyhow!("Blockchain not found")));
                    }

                },
                Msg::GetPrivateAndPublic(tx) => {
                    let _ = tx.send(Ok((self.private_chain.id().clone(), self.public_chain.id().clone())));
                },
                Msg::StartNewConnection { ids_tx, done } => {
                    self.new_connection_web_rtc_ids = WebRtcIds::new();
                    let ids = self.new_connection_web_rtc_ids.clone();

                    let _ = ids_tx.send(Ok(ids.clone()));

                    let data_channel = self
                        .connect_web_rtc_data_channel(ids.own_id.clone(), ids.room_id.clone())
                        .await
                        .ok();

                    let connection = UserConnection {
                        id: blockchain::blockchain::Id::new(),
                        transport: UserConnectionTransport::WebRtc { ids: ids.clone() },
                        data_channel,
                    };

                    let done_result = match connection.write(self.key_db.connection()).await {
                        Ok(()) => {
                            self.user_connections.push(connection);
                            Ok(())
                        }
                        Err(error) => {
                            Err(error)
                        }
                    };

                    let _ = done.send(done_result);
                }
                Msg::CreateNewConnectionFromWebRtcIds { ids, tx } => {
                    let data_channel = self
                        .connect_web_rtc_data_channel(ids.remote_id.clone(), ids.room_id.clone())
                        .await
                        .ok();

                    let connection = UserConnection {
                        id: blockchain::blockchain::Id::new(),
                        transport: UserConnectionTransport::WebRtc { ids: ids.clone() },
                        data_channel,
                    };

                    match connection.write(self.key_db.connection()).await {
                        Ok(()) => {
                            self.user_connections.push(connection);
                            let _ = tx.send(Ok(()));
                        }
                        Err(error) => {
                            let _ = tx.send(Err(error));
                        }
                    }
                }
            }
        }
    }
}