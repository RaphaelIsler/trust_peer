use rand::rngs::OsRng;
use rand::RngCore;

#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Salt(pub [u8; 32]);

impl Salt {
    /// Generate a new random Salt
    pub fn new() -> Self {
        let mut rng = OsRng;
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        Salt(b)
    }

    /// Create a Salt from a 32-byte slice
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes[0..32]);
        Salt(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salt_len() {
        let s = Salt::new();
        assert_eq!(s.0.len(), 32);
    }
}
