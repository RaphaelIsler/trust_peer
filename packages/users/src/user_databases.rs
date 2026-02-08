use anyhow::Result;
use std::path::Path;
use crate::{User, UserId};
use db::DbEntity;

/// Manages the private and public databases of a user
pub struct UserDatabases {
    user_id: UserId,
    private_db: db::DB,
    public_db: db::DB,
}

impl UserDatabases {
    /// Opens or creates both databases for a user
    pub async fn open<P: AsRef<Path>>(base_path: P, user_id: &UserId) -> Result<Self> {
        let base_path = base_path.as_ref();
        let user_dir = base_path.join(user_id.inner().to_string());

        // Create user directory
        std::fs::create_dir_all(&user_dir)?;

        let private_db_path = user_dir.join("private.sqlite");
        let public_db_path = user_dir.join("public.sqlite");

        let private_db = db::DB::open(&private_db_path).await?;
        let public_db = db::DB::open(&public_db_path).await?;

        let mut dbs = Self {
            user_id: user_id.clone(),
            private_db,
            public_db,
        };

        dbs.init_private_db().await?;
        dbs.init_public_db().await?;

        Ok(dbs)
    }

    /// Initializes the private database (contains user info, keys, blockchains)
    async fn init_private_db(&mut self) -> Result<()> {
        // User table via User DbEntity
        self.private_db.migrate_table::<User>().await?;
        self.private_db.migrate_table::<crypto::KeyMeta>().await?;
        self.private_db.migrate_table::<blockchain::Blockchain>().await?;

        Ok(())
    }

    /// Initializes the public database (only blockchains)
    async fn init_public_db(&mut self) -> Result<()> {
        // Blockchains table via Blockchain DbEntity
        self.public_db.migrate_table::<blockchain::Blockchain>().await?;
        Ok(())
    }

    /// Returns reference to the private database
    pub fn private_db(&self) -> &db::DB {
        &self.private_db
    }

    /// Returns reference to the public database
    pub fn public_db(&self) -> &db::DB {
        &self.public_db
    }

    /// Returns the path to the private database
    pub fn private_db_path(&self) -> &Path {
        self.private_db.path()
    }

    /// Returns the path to the public database
    pub fn public_db_path(&self) -> &Path {
        self.public_db.path()
    }

    /// Saves user information to the private database
    pub async fn save_user_info(&self, user: &User) -> Result<()> {
        user.write(self.private_db.connection()).await?;
        Ok(())
    }

    /// Loads user information from the private database
    pub async fn load_user_info(&self) -> Result<Option<User>> {
        User::read(self.private_db.connection(), &self.user_id).await
    }

    /// Creates a new ed25519 key pair and saves it to the private database
    pub async fn add_key_pair(&self) -> Result<crypto::KeyMeta> {
        crypto::KeyMeta::create_ed25519(self.private_db.connection()).await
    }

    /// Lists all keys (from private database)
    pub async fn list_keys(&self) -> Result<Vec<crypto::KeyMeta>> {
        crypto::KeyMeta::list(self.private_db.connection()).await
    }
}
