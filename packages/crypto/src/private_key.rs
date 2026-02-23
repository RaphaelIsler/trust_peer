#[cfg(feature = "ed25519")]
use ed25519_dalek::{SigningKey, Signer, SECRET_KEY_LENGTH};
use rand::RngCore;
use rand::rngs::OsRng;
use sqlx::{Decode, Encode, Sqlite, Type};
use sqlx::encode::IsNull;
use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use std::borrow::Cow;
use std::error::Error as StdError;

/// Private Key structure for ed25519 (32 bytes secret key)
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PrivateKey(pub Vec<u8>);

impl PrivateKey {
    /// Generate a new random ed25519 private key
    #[cfg(feature = "ed25519")]
    pub fn new() -> Self {
        let mut secret_bytes = [0u8; SECRET_KEY_LENGTH];
        OsRng.fill_bytes(&mut secret_bytes);
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        PrivateKey(signing_key.to_bytes().to_vec())
    }

    /// Create a PrivateKey from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        PrivateKey(bytes.to_vec())
    }

    /// Get the bytes of the private key
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Get the corresponding public key
    #[cfg(feature = "ed25519")]
    pub fn public_key(&self) -> crate::PublicKey {
        let signing_key = SigningKey::from_bytes(
            self.0.as_slice().try_into()
                .expect("Invalid private key length")
        );
        let verifying_key = signing_key.verifying_key();
        crate::PublicKey(verifying_key.to_bytes().to_vec())
    }

    /// Sign data with this private key
    #[cfg(feature = "ed25519")]
    pub fn sign(&self, data: &[u8]) -> Result<crate::Signature, anyhow::Error> {
        let signing_key = SigningKey::from_bytes(
            self.0.as_slice().try_into()
                .map_err(|_| anyhow::anyhow!("Invalid private key length"))?
        );
        let sig = signing_key.sign(data);
        Ok(crate::Signature(sig.to_bytes().to_vec()))
    }

    /// Decrypt data that was encrypted for this key's public key
    #[cfg(feature = "ed25519")]
    pub fn decrypt(&self, sealed: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        crate::sealed_box::open(self.as_bytes(), sealed)
    }
}

impl Type<Sqlite> for PrivateKey {
    fn type_info() -> SqliteTypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <Vec<u8> as Type<Sqlite>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Sqlite> for PrivateKey {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let bytes = <Vec<u8> as Decode<Sqlite>>::decode(value)?;
        Ok(PrivateKey(bytes))
    }
}

impl<'q> Encode<'q, Sqlite> for PrivateKey {
    fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> Result<IsNull, Box<dyn StdError + Send + Sync>> {
        args.push(SqliteArgumentValue::Blob(Cow::Owned(
            self.0.to_vec(),
        )));
        Ok(IsNull::No)
    }
}
