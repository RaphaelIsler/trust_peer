use crate::Block;
use crypto::{Salt, Signature};

pub type Id = helper::UId<Block>;


#[derive(serde::Serialize, serde::Deserialize)]
pub enum BlockEntry{
    Verification{salt: Salt, public: Vec<u8>, signature: Signature},
    Identification{data: Vec<u8>},
}
