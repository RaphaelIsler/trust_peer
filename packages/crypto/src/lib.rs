//! Simple crypto utilities: checksums, key generation, storage and signing.

use anyhow::Result;
use tokio_rusqlite::Connection;

mod hash;
pub use hash::Hash;

mod salt;
pub use salt::Salt;

mod private_key;
pub use private_key::PrivateKey;

mod public_key;
pub use public_key::PublicKey;

mod signature;
pub use signature::Signature;

mod key_meta;
pub use key_meta::KeyMeta;

pub struct KeyStore {
    conn: Connection,
}

impl KeyStore {
    /// Open or create a sqlite database at the given path
    pub async fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path).await?;
        KeyMeta::init_table(&conn).await?;
        Ok(Self { conn })
    }

    /// Create a new keypair (ed25519) and store it in the DB. Returns KeyMeta.
    #[cfg(feature = "ed25519")]
    pub async fn create_ed25519_key(&self) -> Result<KeyMeta> {
        KeyMeta::create_ed25519(&self.conn).await
    }

    /// Load a key by id
    pub async fn load_key(&self, id: &str) -> Result<Option<KeyMeta>> {
        KeyMeta::load(&self.conn, id).await
    }

    /// Sign data using the stored key
    pub async fn sign(&self, id: &str, data: &[u8]) -> Result<Signature> {
        let key = self.load_key(id).await?
            .ok_or_else(|| anyhow::anyhow!("Key not found"))?;

        match key.alg.as_str() {
            "ed25519" => {
                #[cfg(feature = "ed25519")]
                {
                    return key.private_key.sign(data);
                }

                #[cfg(not(feature = "ed25519"))]
                {
                    anyhow::bail!("ed25519 support is not compiled into this crate");
                }
            }
            other => {
                anyhow::bail!("Algorithm not supported: {}", other);
            }
        }
    }

    /// Verify a signature using stored public key
    pub async fn verify(&self, id: &str, data: &[u8], signature: &Signature) -> Result<bool> {
        let key = self.load_key(id).await?
            .ok_or_else(|| anyhow::anyhow!("Key not found"))?;

        match key.alg.as_str() {
            "ed25519" => {
                #[cfg(feature = "ed25519")]
                {
                    match key.public_key.verify(data, signature) {
                        Ok(()) => Ok(true),
                        Err(_) => Ok(false),
                    }
                }

                #[cfg(not(feature = "ed25519"))]
                {
                    anyhow::bail!("ed25519 support is not compiled into this crate");
                }
            }
            other => {
                anyhow::bail!("Algorithm not supported: {}", other);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_checksum() {
        let data = b"hello world";
        let hash = Hash::new(data);
        assert_eq!(hash.0.len(), 32);
    }

    #[cfg(feature = "ed25519")]
    #[tokio::test]
    async fn test_create_sign_verify() {
        let tmp = NamedTempFile::new().unwrap();
        let store = KeyStore::new(tmp.path().to_str().unwrap()).await.unwrap();
        let meta = store.create_ed25519_key().await.unwrap();
        let msg = b"the quick brown fox";
        let sig = store.sign(&meta.id, msg).await.unwrap();
        let ok = store.verify(&meta.id, msg, &sig).await.unwrap();
        assert!(ok);
    }
}
