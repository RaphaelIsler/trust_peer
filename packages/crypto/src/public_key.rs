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
        use ed25519_dalek::{Verifier, VerifyingKey, Signature as DalekSignature};

        let verifying_key = VerifyingKey::from_bytes(
            self.0.as_slice().try_into()
                .map_err(|_| anyhow::anyhow!("Invalid public key length"))?
        )?;

        let sig = DalekSignature::from_bytes(
            signature.as_bytes().try_into()
                .map_err(|_| anyhow::anyhow!("Invalid signature length"))?
        );

        verifying_key.verify(data, &sig)?;
        Ok(())
    }
}
