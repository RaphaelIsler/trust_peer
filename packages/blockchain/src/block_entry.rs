use crypto::{Salt, Signature};
use anyhow::Result;
use super::block_link::BlockLink;


pub type Id = helper::UId<BlockEntry>;


#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum BlockEntry{
    Verification{salt: Salt, public: crypto::PublicKey, signature: Signature},
    Identification{data: Vec<u8>},
    Link(BlockLink),
}


impl BlockEntry{
    pub fn new_verification(key: &crypto::KeyMeta) -> Result<Self> {
        let salt = Salt::new();
        let signature = Signature::new(&key.private_key, &salt.0)?;
        Ok(Self::Verification {
            salt,
            public: key.public_key.clone(),
            signature,
        })
    }
}