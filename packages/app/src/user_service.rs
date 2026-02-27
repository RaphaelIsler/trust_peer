use anyhow::Result;
use chrono::NaiveDate;
use blockchain::Blockchain;
use db::DbEntity;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tokio::task::JoinHandle;
 use tokio::sync::oneshot;
use crate::{KeyValue, User, UserId};

#[derive(helper::ServiceWrapper)]
pub enum Msg {
    GetPrivateAndPublic(oneshot::Sender<Result<(blockchain::blockchain::Id, blockchain::blockchain::Id)>>),
    GetBlocks{id: blockchain::blockchain::Id, count: usize, start_at: Option<usize>, tx: oneshot::Sender<Result<Vec<blockchain::Block>>>},
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
    }
}

struct Internal{
    user_id: UserId,
    private_chain_db: db::DB,
    key_db: db::DB,
    public_chain_db: db::DB,
    private_chain: Blockchain,
    public_chain: Blockchain,
}


impl Internal {
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

        let private_pub = ret.add_key_pair(&ret.private_chain.id().inner()).await?;
        let fist_private = blockchain::BlockEntry::new_verification(&private_pub)?;
        let first_block = ret.private_chain.append_entries_with_db(vec![fist_private], &ret.private_chain_db).await?;

        let public_pub = ret.add_key_pair(&ret.public_chain.id().inner()).await?;
        let fist_public = blockchain::BlockEntry::new_verification(&public_pub)?;
        let first_link = blockchain::BlockLink::from_block(ret.private_chain.id(), &first_block)?;


        ret.public_chain.append_entries_with_db(vec![fist_public, blockchain::BlockEntry::from_link(first_link)?], &ret.public_chain_db).await?;
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

        let private_chain = Blockchain::from_db(&mut private_chain_db).await?;
        let public_chain = Blockchain::from_db(&mut public_chain_db).await?;

        Ok(Self {
            user_id: user_id.clone(),
            private_chain_db,
            public_chain_db,
            key_db,
            private_chain,
            public_chain,
        })
    }



    /// Initializes the private database (contains user info, keys, blockchains)
    async fn init_private_db(private_db: &mut db::DB) -> Result<()> {
        // User table via User DbEntity
        private_db.migrate_table::<blockchain::Block>().await?;

        Ok(())
    }
    async fn init_key_db(private_db: &mut db::DB) -> Result<()> {
        // User table via User DbEntity
        private_db.migrate_table::<crypto::KeyMeta>().await?;
        private_db.migrate_table::<KeyValue>().await?;

        Ok(())
    }

    /// Initializes the public database (only blockchains)
    async fn init_public_db(public_db: &mut db::DB) -> Result<()> {
        // Blockchains table via Blockchain DbEntity
        public_db.migrate_table::<blockchain::Block>().await?;
        Ok(())
    }


    async fn add_key_pair(&self, id: &uuid::Uuid) -> Result<crypto::KeyMeta> {
        crypto::KeyMeta::create_ed25519(id, self.key_db.connection()).await
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
                }
            }
        }
    }
}