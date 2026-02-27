use sha2::{Digest, Sha256};

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create a new SHA-256 hash from bytes
    pub fn new(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let result = hasher.finalize();

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Self(hash)
    }

    pub fn empty() -> Self {
        Self([0u8; 32])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_len() {
        let h = Hash::new(b"hello");
        assert_eq!(h.0.len(), 32);
    }
}
