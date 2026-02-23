use anyhow::Result;
use chrono::NaiveDate;
use blockchain::Blockchain;
use db::DbEntity;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use tokio::task::JoinHandle;

use crate::{KeyValue, User, UserId};

pub enum Msg {}

pub struct Service {
    user_id: UserId,
    private_chain_db: db::DB,
    key_db: db::DB,
    public_chain_db: db::DB,
    private_chain: Blockchain,
    public_chain: Blockchain,
    rx: Option<tokio::sync::mpsc::Receiver<Msg>>,
}

static SERVICE_REGISTRY: OnceLock<Mutex<HashMap<String, JoinHandle<()>>>> = OnceLock::new();

fn registry() -> &'static Mutex<HashMap<String, JoinHandle<()>>> {
    SERVICE_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

impl Service {
    /// Opens the user service with a message receiver (used for per-user workers)
    pub async fn open<P: AsRef<Path>>(
        base_path: P,
        user_id: &UserId,
        rx: tokio::sync::mpsc::Receiver<Msg>,
    ) -> Result<Self> {
        Self::open_internal(base_path, user_id, Some(rx)).await
    }

    /// Opens databases without starting the message processing loop
    pub async fn open_databases<P: AsRef<Path>>(base_path: P, user_id: &UserId) -> Result<Self> {
        Self::open_internal(base_path, user_id, None).await
    }

    pub async fn create_new_instance(
        base_path: impl AsRef<Path>,
        user: &User,
        middle_name: Option<String>,
        date_of_birth: NaiveDate,
    ) -> Result<Self> {
        let user_id = user.id.clone();

        let mut ret = Self::open_internal(base_path, &user_id, None).await?;
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
        let public_pub = ret.add_key_pair(&ret.public_chain.id().inner()).await?;
        let fist_private = blockchain::BlockEntry::new_verification(&private_pub)?;
        let fist_public = blockchain::BlockEntry::new_verification(&public_pub)?;

        ret.public_chain.append_entries_with_db(vec![fist_public], &ret.public_chain_db).await?;
        ret.private_chain.append_entries_with_db(vec![fist_private], &ret.private_chain_db).await?;
        Ok(ret)
    }

    /// Starts a background service for the given user (no-op if already running)
    pub fn start_background(base_path: PathBuf, user_id: UserId) {
        let id_key = user_id.inner().to_string();

        let mut map = registry().lock().expect("service registry lock poisoned");
        if map.contains_key(&id_key) {
            return;
        }

        let task_user_id = user_id.clone();
        let task_base_path = base_path.clone();
        let task_key = id_key.clone();

        let handle = tokio::spawn(async move {
            match Service::open_databases(task_base_path, &task_user_id).await {
                Ok(service) => service.run().await,
                Err(e) => eprintln!("Failed to start user service for {task_user_id:?}: {e}"),
            }

            if let Ok(mut map) = registry().lock() {
                map.remove(&task_key);
            }
        });

        map.insert(id_key, handle);
    }

    async fn open_internal<P: AsRef<Path>>(
        base_path: P,
        user_id: &UserId,
        rx: Option<tokio::sync::mpsc::Receiver<Msg>>,
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
            rx,
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


    pub async fn run(mut self) {
        self.process().await;
    }

    async fn process(&mut self) {
        // TODO: Aufträge abarbeiten
        let _ = self.rx.take();
    }
}