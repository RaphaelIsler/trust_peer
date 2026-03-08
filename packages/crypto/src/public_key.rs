use sqlx::encode::IsNull;
use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Encode, Sqlite, Type};
use std::borrow::Cow;
use std::error::Error as StdError;

/// Public Key structure for ed25519 (32 bytes)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PublicKey(pub Vec<u8>);

impl PublicKey {
    /// Create a PublicKey from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        PublicKey(bytes.to_vec())
    }

    /// Get the bytes of the public key
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Verify a signature
    #[cfg(feature = "ed25519")]
    pub fn verify(&self, data: &[u8], signature: &crate::Signature) -> Result<(), anyhow::Error> {
        use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};

        let verifying_key = VerifyingKey::from_bytes(
            self.0
                .as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid public key length"))?,
        )?;

        let sig = DalekSignature::from_bytes(
            signature
                .as_bytes()
                .try_into()
                .map_err(|_| anyhow::anyhow!("Invalid signature length"))?,
        );

        verifying_key.verify(data, &sig)?;
        Ok(())
    }

    /// Encrypt data for this public key
    #[cfg(feature = "ed25519")]
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        crate::sealed_box::seal(self.as_bytes(), plaintext)
    }
}

impl Type<Sqlite> for PublicKey {
    fn type_info() -> SqliteTypeInfo {
        <Vec<u8> as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &SqliteTypeInfo) -> bool {
        <Vec<u8> as Type<Sqlite>>::compatible(ty)
    }
}

impl<'r> Decode<'r, Sqlite> for PublicKey {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let bytes = <Vec<u8> as Decode<Sqlite>>::decode(value)?;
        Ok(PublicKey(bytes))
    }
}

impl<'q> Encode<'q, Sqlite> for PublicKey {
    fn encode_by_ref(
        &self,
        args: &mut Vec<SqliteArgumentValue<'q>>,
    ) -> Result<IsNull, Box<dyn StdError + Send + Sync>> {
        args.push(SqliteArgumentValue::Blob(Cow::Owned(self.0.to_vec())));
        Ok(IsNull::No)
    }
}
