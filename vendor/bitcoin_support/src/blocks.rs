use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Blocks(u32);

impl Blocks {
    pub const fn new(num_blocks: u32) -> Self {
        Blocks(num_blocks)
    }
}

impl From<Duration> for Blocks {
    fn from(duration: Duration) -> Self {
        let seconds = duration.as_secs();
        // ~10 minutes = ~600 seconds blocks
        let blocks = seconds / 600;
        let blocks = if blocks > ::std::u32::MAX as u64 {
            ::std::u32::MAX
        } else {
            blocks as u32
        };
        match seconds % 600 {
            0 => Blocks(blocks),
            _ => Blocks(blocks + 1),
        }
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
