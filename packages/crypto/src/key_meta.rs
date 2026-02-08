use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[cfg(feature = "ed25519")]
use ed25519_dalek::{SigningKey, SECRET_KEY_LENGTH};
use rand::RngCore;
use rand::rngs::OsRng;

use crate::{PrivateKey, PublicKey};

/// Key metadata structure for storing keys in SQLite
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyMeta {
    pub id: String,
    pub alg: String,
    pub public_key: PublicKey,
    pub private_key: PrivateKey,
}

impl KeyMeta {
    #[cfg(feature = "ed25519")]
    pub async fn create_ed25519(conn: &SqlitePool) -> Result<Self> {
        let mut secret_bytes = [0u8; SECRET_KEY_LENGTH];
        OsRng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let id = Uuid::new_v4().to_string();

        let public_key = PublicKey::from_bytes(&signing_key.verifying_key().to_bytes());
        let private_key = PrivateKey::from_bytes(&signing_key.to_bytes());

        let id_clone = id.clone();
        let public_key_clone = public_key.clone();
        let private_key_clone = private_key.clone();

        sqlx::query(
            "INSERT INTO keys (id, alg, public_key, private_key) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(id_clone)
        .bind("ed25519")
        .bind(public_key_clone)
        .bind(private_key_clone)
        .execute(conn)
        .await?;

        Ok(KeyMeta {
            id,
            alg: "ed25519".to_string(),
            public_key,
            private_key,
        })
    }

    /// Initialize the keys table in the database
    pub async fn init_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS keys (
                id TEXT PRIMARY KEY,
                alg TEXT NOT NULL,
                public_key BLOB NOT NULL,
                private_key BLOB NOT NULL
            );",
        )
        .execute(conn)
        .await?;
        Ok(())
    }
}

impl db::DbEntity for KeyMeta {
    type Id = String;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn table_name() -> &'static str {
        "keys"
    }

    fn schema_version() -> u32 {
        1
    }

    async fn create_table(conn: &SqlitePool) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS keys (
                id TEXT PRIMARY KEY,
                alg TEXT NOT NULL,
                public_key BLOB NOT NULL,
                private_key BLOB NOT NULL
            );",
        )
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn update_table(_conn: &SqlitePool, _from_version: u32, _to_version: u32) -> Result<()> {
        // Keine Migrationen erforderlich (noch)
        Ok(())
    }

    async fn write(&self, conn: &SqlitePool) -> Result<()> {
        let id = self.id.clone();
        let alg = self.alg.clone();
        let public_key = self.public_key.clone();
        let private_key = self.private_key.clone();

        sqlx::query(
            "INSERT OR REPLACE INTO keys (id, alg, public_key, private_key) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(id)
        .bind(alg)
        .bind(public_key)
        .bind(private_key)
        .execute(conn)
        .await?;
        Ok(())
    }

    async fn read(conn: &SqlitePool, id: &Self::Id) -> Result<Option<Self>> {
        let id = id.clone();

        let row = sqlx::query("SELECT alg, public_key, private_key FROM keys WHERE id = ?1")
            .bind(&id)
            .fetch_optional(conn)
            .await?;

        if let Some(row) = row {
            Ok(Some(KeyMeta {
                id,
                alg: row.try_get(0)?,
                public_key: row.try_get(1)?,
                private_key: row.try_get(2)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(conn: &SqlitePool, id: &Self::Id) -> Result<()> {
        let id = id.clone();
        sqlx::query("DELETE FROM keys WHERE id = ?1")
            .bind(id)
            .execute(conn)
            .await?;
        Ok(())
    }

    async fn list(conn: &SqlitePool) -> Result<Vec<Self>> {
        let rows = sqlx::query("SELECT id, alg, public_key, private_key FROM keys")
            .fetch_all(conn)
            .await?;

        let mut keys = Vec::with_capacity(rows.len());
        for row in rows {
            keys.push(KeyMeta {
                id: row.try_get(0)?,
                alg: row.try_get(1)?,
                public_key: row.try_get(2)?,
                private_key: row.try_get(3)?,
            });
        }

        Ok(keys)
    }
}
