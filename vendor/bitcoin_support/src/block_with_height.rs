use bitcoin::Block;

#[derive(Debug)]
pub struct BlockWithHeight {
    pub block: Block,
    pub height: u32,
}
