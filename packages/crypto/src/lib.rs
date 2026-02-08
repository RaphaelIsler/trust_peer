//! Simple crypto utilities: checksums, key generation, storage and signing.

use anyhow::Result;

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

}
