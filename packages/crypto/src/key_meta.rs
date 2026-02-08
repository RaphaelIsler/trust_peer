use anyhow::Result;
use tokio_rusqlite::Connection;
use tokio_rusqlite::rusqlite;
use uuid::Uuid;

#[cfg(feature = "ed25519")]
use ed25519_dalek::{SigningKey, SECRET_KEY_LENGTH};
use rand::RngCore;
use rand::rngs::OsRng;

use crate::{PrivateKey, PublicKey};

/// Key metadata structure for storing keys in SQLite
#[derive(Debug, Clone)]
pub struct KeyMeta {
    pub id: String,
    pub alg: String,
    pub public_key: PublicKey,
    pub private_key: PrivateKey,
}

impl KeyMeta {
    /// Create a new ed25519 keypair and store it in the database
    #[cfg(feature = "ed25519")]
    pub async fn create_ed25519(conn: &Connection) -> Result<Self> {
        let mut secret_bytes = [0u8; SECRET_KEY_LENGTH];
        OsRng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let id = Uuid::new_v4().to_string();

        let public_key = PublicKey::from_bytes(&signing_key.verifying_key().to_bytes());
        let private_key = PrivateKey::from_bytes(&signing_key.to_bytes());

        let id_clone = id.clone();
        let pub_bytes = public_key.as_bytes().to_vec();
        let priv_bytes = private_key.as_bytes().to_vec();

        conn.call(move |conn| {
            conn.execute(
                "INSERT INTO keys (id, alg, public_key, private_key) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id_clone, "ed25519", pub_bytes, priv_bytes],
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;

        Ok(KeyMeta {
            id,
            alg: "ed25519".to_string(),
            public_key,
            private_key,
        })
    }

    /// Load a key by id from the database
    pub async fn load(conn: &Connection, id: &str) -> Result<Option<Self>> {
        let id = id.to_string();

        let result = conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT alg, public_key, private_key FROM keys WHERE id = ?1")?;
                let mut rows = stmt.query([&id])?;

                if let Some(row) = rows.next()? {
                    let alg: String = row.get(0)?;
                    let pubk: Vec<u8> = row.get(1)?;
                    let privk: Vec<u8> = row.get(2)?;
                    Ok::<Option<(String, String, Vec<u8>, Vec<u8>)>, rusqlite::Error>(Some((id, alg, pubk, privk)))
                } else {
                    Ok(None)
                }
            })
            .await?;

        Ok(result.map(|(id, alg, pubk, privk)| KeyMeta {
            id,
            alg,
            public_key: PublicKey::from_bytes(&pubk),
            private_key: PrivateKey::from_bytes(&privk),
        }))
    }

    /// Initialize the keys table in the database
    pub async fn init_table(conn: &Connection) -> Result<()> {
        conn.call(|conn| {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS keys (
                    id TEXT PRIMARY KEY,
                    alg TEXT NOT NULL,
                    public_key BLOB NOT NULL,
                    private_key BLOB NOT NULL
                );",
            )?;
            Ok::<(), rusqlite::Error>(())
        })
        .await?;
        Ok(())
    }
}
