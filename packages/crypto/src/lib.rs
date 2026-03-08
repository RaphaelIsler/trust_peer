//! Simple crypto utilities: checksums, key generation, storage and signing.

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

mod sealed_box;

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
    #[test]
    fn test_sealed_box_roundtrip() {
        let private_key = PrivateKey::new();
        let public_key = private_key.public_key();

        let plaintext = b"secret message";
        let sealed = public_key.encrypt(plaintext).expect("seal");
        let opened = private_key.decrypt(&sealed).expect("open");

        assert_eq!(opened, plaintext);
    }
}
