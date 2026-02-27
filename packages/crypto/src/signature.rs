#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    /// Create a new signature by signing data with a private key
    #[cfg(feature = "ed25519")]
    pub fn new(private_key: &crate::PrivateKey, data: &[u8]) -> Result<Self, anyhow::Error> {
        private_key.sign(data)
    }

    pub fn from_bytes(b: &[u8]) -> Self {
        Signature(b.to_vec())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
