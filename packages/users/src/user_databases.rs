use anyhow::Result;
use std::path::Path;
use crate::{User, UserId};
use db::DbEntity;

/// Verwaltet die private und öffentliche Datenbank eines Benutzers
pub struct UserDatabases {
    user_id: UserId,
    private_db: db::DB,
    public_db: db::DB,
}

impl UserDatabases {
    /// Öffnet oder erstellt beide Datenbanken für einen Benutzer
    pub async fn open<P: AsRef<Path>>(base_path: P, user_id: &UserId) -> Result<Self> {
        let base_path = base_path.as_ref();
        let user_dir = base_path.join(user_id.inner().to_string());

        // Erstelle Benutzer-Verzeichnis
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

    /// Initialisiert die private Datenbank (enthält Benutzerinfo, Keys, Blockchains)
    async fn init_private_db(&mut self) -> Result<()> {
        // User Tabelle über User DbEntity
        self.private_db.migrate_table::<User>().await?;
        self.private_db.migrate_table::<crypto::KeyMeta>().await?;
        self.private_db.migrate_table::<blockchain::Blockchain>().await?;

        Ok(())
    }

    /// Initialisiert die öffentliche Datenbank (nur Blockchains)
    async fn init_public_db(&mut self) -> Result<()> {
        // Blockchains Tabelle über Blockchain DbEntity
        self.public_db.migrate_table::<blockchain::Blockchain>().await?;
        Ok(())
    }

    /// Gibt Referenz zur privaten Datenbank zurück
    pub fn private_db(&self) -> &db::DB {
        &self.private_db
    }

    /// Gibt Referenz zur öffentlichen Datenbank zurück
    pub fn public_db(&self) -> &db::DB {
        &self.public_db
    }

    /// Gibt den Pfad zur privaten DB zurück
    pub fn private_db_path(&self) -> &Path {
        self.private_db.path()
    }

    /// Gibt den Pfad zur öffentlichen DB zurück
    pub fn public_db_path(&self) -> &Path {
        self.public_db.path()
    }

    /// Speichert Benutzerinformationen in der privaten DB
    pub async fn save_user_info(&self, user: &User) -> Result<()> {
        user.write(self.private_db.connection()).await?;
        Ok(())
    }

    /// Lädt Benutzerinformationen aus der privaten DB
    pub async fn load_user_info(&self) -> Result<Option<User>> {
        User::read(self.private_db.connection(), &self.user_id).await
    }

    /// Erstellt ein neues ed25519 Schlüsselpaar und speichert es in der privaten DB
    pub async fn add_key_pair(&self) -> Result<crypto::KeyMeta> {
        crypto::KeyMeta::create_ed25519(self.private_db.connection()).await
    }

    /// Listet alle Schlüssel auf (aus private DB)
    pub async fn list_keys(&self) -> Result<Vec<crypto::KeyMeta>> {
        crypto::KeyMeta::list(self.private_db.connection()).await
    }
}
