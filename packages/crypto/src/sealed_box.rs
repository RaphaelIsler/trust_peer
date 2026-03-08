use anyhow::Result;

#[cfg(feature = "ed25519")]
use chacha20poly1305::aead::{Aead, KeyInit};
#[cfg(feature = "ed25519")]
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
#[cfg(feature = "ed25519")]
use curve25519_dalek::edwards::CompressedEdwardsY;
#[cfg(feature = "ed25519")]
use hkdf::Hkdf;
#[cfg(feature = "ed25519")]
use rand::rngs::OsRng;
#[cfg(feature = "ed25519")]
use rand::RngCore;
#[cfg(feature = "ed25519")]
use sha2::{Digest, Sha256, Sha512};
#[cfg(feature = "ed25519")]
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};

const SEALED_BOX_VERSION: u8 = 1;
const EPHEMERAL_KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const HEADER_LEN: usize = 1 + EPHEMERAL_KEY_LEN + NONCE_LEN;

#[cfg(feature = "ed25519")]
pub fn seal(public_key_bytes: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let recipient_public = ed25519_public_to_x25519(public_key_bytes)?;

    let ephemeral_secret = X25519Secret::random_from_rng(OsRng);
    let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

    let shared = ephemeral_secret.diffie_hellman(&recipient_public);
    let key = derive_key(shared.as_bytes(), ephemeral_public.as_bytes())?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), plaintext)
        .map_err(|_| anyhow::anyhow!("failed to encrypt payload"))?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.push(SEALED_BOX_VERSION);
    out.extend_from_slice(ephemeral_public.as_bytes());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

#[cfg(feature = "ed25519")]
pub fn open(private_key_bytes: &[u8], sealed: &[u8]) -> Result<Vec<u8>> {
    if sealed.len() < HEADER_LEN {
        anyhow::bail!("sealed payload too short");
    }

    let version = sealed[0];
    if version != SEALED_BOX_VERSION {
        anyhow::bail!("unsupported sealed box version: {version}");
    }

    let ephemeral_public_bytes = &sealed[1..1 + EPHEMERAL_KEY_LEN];
    let nonce_bytes = &sealed[1 + EPHEMERAL_KEY_LEN..HEADER_LEN];
    let ciphertext = &sealed[HEADER_LEN..];

    let recipient_secret = ed25519_private_to_x25519(private_key_bytes)?;
    let mut ephemeral_public_array = [0u8; EPHEMERAL_KEY_LEN];
    ephemeral_public_array.copy_from_slice(ephemeral_public_bytes);
    let ephemeral_public = X25519PublicKey::from(ephemeral_public_array);

    let shared = recipient_secret.diffie_hellman(&ephemeral_public);
    let key = derive_key(shared.as_bytes(), ephemeral_public.as_bytes())?;

    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .map_err(|_| anyhow::anyhow!("failed to decrypt payload"))?;

    Ok(plaintext)
}

#[cfg(feature = "ed25519")]
fn derive_key(shared_secret: &[u8], ephemeral_public: &[u8]) -> Result<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(Some(ephemeral_public), shared_secret);
    let mut key = [0u8; 32];
    hkdf.expand(b"crypto-sealed-box", &mut key)
        .map_err(|_| anyhow::anyhow!("hkdf expansion failed"))?;
    Ok(key)
}

#[cfg(feature = "ed25519")]
fn ed25519_public_to_x25519(public_key_bytes: &[u8]) -> Result<X25519PublicKey> {
    let bytes: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid public key length"))?;

    let compressed = CompressedEdwardsY(bytes);
    let edwards = compressed
        .decompress()
        .ok_or_else(|| anyhow::anyhow!("invalid ed25519 public key"))?;

    let montgomery = edwards.to_montgomery();
    Ok(X25519PublicKey::from(montgomery.to_bytes()))
}

#[cfg(feature = "ed25519")]
fn ed25519_private_to_x25519(private_key_bytes: &[u8]) -> Result<X25519Secret> {
    let bytes: [u8; 32] = private_key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid private key length"))?;

    let hash = Sha512::digest(bytes);
    let mut secret_bytes = [0u8; 32];
    secret_bytes.copy_from_slice(&hash[..32]);

    secret_bytes[0] &= 248;
    secret_bytes[31] &= 127;
    secret_bytes[31] |= 64;

    Ok(X25519Secret::from(secret_bytes))
}
