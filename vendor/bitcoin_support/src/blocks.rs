#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Blocks(u32);

pub const BTC_BLOCKS_IN_24H: Blocks = Blocks::new(24 * 60 / 10);

impl Blocks {
    pub const fn new(num_blocks: u32) -> Self {
        Blocks(num_blocks)
    }
}

impl Default for Blocks {
    fn default() -> Self {
        BTC_BLOCKS_IN_24H
    }
}

impl From<u32> for Blocks {
    fn from(num: u32) -> Self {
        Blocks::new(num)
    }
}

impl From<Blocks> for u32 {
    fn from(blocks: Blocks) -> u32 {
        blocks.0
    }
}
