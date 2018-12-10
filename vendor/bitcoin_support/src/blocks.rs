use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Blocks(pub u32);

pub const BTC_BLOCKS_IN_24H: Blocks = Blocks::new(24 * 60 / 10);

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
        let blocks = if blocks > u64::from(::std::u32::MAX) {
            ::std::u32::MAX - 1 // 600 does not devise u32::MAX
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

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn given_one_hour_returns_six_blocks() {
        let duration = Duration::from_secs(60 * 60);
        let blocks: Blocks = duration.into();
        let blocks: u32 = blocks.into();

        assert_that(&blocks).is_equal_to(6);
    }

    #[test]
    fn given_fifteen_min_returns_two_blocks() {
        let duration = Duration::from_secs(60 * 15);
        let blocks: Blocks = duration.into();
        let blocks: u32 = blocks.into();

        assert_that(&blocks).is_equal_to(2);
    }

    #[test]
    fn given_above_u32_limit_returns_u32_limit() {
        let seconds = ::std::u64::MAX;
        let duration = Duration::from_secs(seconds);
        let blocks: Blocks = duration.into();
        let blocks: u32 = blocks.into();

        assert_that(&blocks).is_equal_to(::std::u32::MAX);
    }

}
