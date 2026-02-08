
use super::Block;
pub type Id = helper::UId<Blockchain>;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Blockchain{
    id: Id,
    blocks: Vec<Block>,
}

impl Blockchain{
    pub fn new() -> Self{
        Self{
            id: Id::new(),
            blocks: vec![Block::init()],
        }
    }
}
