pub mod bitcoin;
pub mod ethereum;
mod jsonrpc;

use crate::Never;
use anyhow::Result;
use async_trait::async_trait;
use genawaiter::sync::{Co, Gen};
use std::{collections::HashSet, future::Future, hash::Hash, time::Duration};
use time::OffsetDateTime;

#[async_trait]
pub trait LatestBlock: Send + Sync + 'static {
    type Block;

    async fn latest_block(&self) -> Result<Self::Block>;
}

#[async_trait]
pub trait BlockByHash: Send + Sync + 'static {
    type Block;
    type BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block>;
}

#[async_trait]
pub trait ConnectedNetwork: Send + Sync + 'static {
    type Network;

    async fn connected_network(&self) -> Result<Self::Network>;
}

/// Checks if a given block predates a certain timestamp.
pub trait Predates {
    fn predates(&self, timestamp: OffsetDateTime) -> bool;
}

/// Abstracts over the ability of getting the hash of the current block.
pub trait BlockHash {
    type BlockHash;

    fn block_hash(&self) -> Self::BlockHash;
}

/// Abstracts over the ability of getting the hash of the previous block.
pub trait PreviousBlockHash {
    type BlockHash;

    fn previous_block_hash(&self) -> Self::BlockHash;
}

/// Fetch blocks from a given timestamp on.
///
/// To do this reliably, we start with the current latest block and walk the
/// blockchain backwards until we pass the given timestamp.
///
/// To make sure we don't miss any blocks as we keep fetching the latest block,
/// we continuously check if we've seen a block's parent before. If we don't we
/// walk back the ancestor chain again until we've seen a parent or we are past
/// the given timestamp again.
pub fn fetch_blocks_since<'a, C, B, H>(
    connector: &'a C,
    start_of_swap: OffsetDateTime,
    poll_interval: Duration,
) -> Gen<B, (), impl Future<Output = Result<Never>> + 'a>
where
    C: LatestBlock<Block = B> + BlockByHash<Block = B, BlockHash = H>,
    B: Predates + BlockHash<BlockHash = H> + PreviousBlockHash<BlockHash = H> + Clone + 'a,
    H: Eq + Hash + Copy,
{
    Gen::new(|co| async move {
        let block = connector.latest_block().await?;

        // Look back in time until we get a block that predates start_of_swap.
        let mut seen_blocks = walk_back_until(
            predates_start_of_swap(start_of_swap),
            block,
            |_| true, // initially, yield all blocks because we haven't seen any of them
            connector,
            poll_interval,
            &co,
        )
        .await?;

        // Look forward in time, but keep going back for missed blocks
        loop {
            let block = connector.latest_block().await?;

            let missed_blocks = walk_back_until(
                seen_block_or_predates_start_of_swap(&seen_blocks, start_of_swap),
                block,
                |b| !seen_blocks.contains(b), // only yield if we haven't seen the block before
                connector,
                poll_interval,
                &co,
            )
            .await?;

            seen_blocks.extend(missed_blocks);

            tokio::time::delay_for(poll_interval).await;
        }
    })
}

/// Walks the blockchain backwards from the given hash until the predicate given
/// in `stop_condition` returns `true`.
///
/// This function yields all blocks as part of its process.
/// This function returns the block-hashes of all visited blocks.
async fn walk_back_until<C, P, Y, B, H>(
    should_stop_here: P,
    starting_block: B,
    should_yield: Y,
    connector: &C,
    poll_interval: Duration,
    co: &Co<B>,
) -> Result<HashSet<H>>
where
    C: BlockByHash<Block = B, BlockHash = H> + LatestBlock<Block = B>,
    P: Fn(&B) -> bool,
    Y: Fn(&H) -> bool,
    B: BlockHash<BlockHash = H> + PreviousBlockHash<BlockHash = H>,
    H: Eq + Hash + Copy,
{
    let mut seen_blocks = HashSet::new();

    let mut current_blockhash = starting_block.block_hash();
    let mut current_block = starting_block;

    let mut delay_until_fetch_latest_block_again = tokio::time::delay_for(poll_interval);

    loop {
        seen_blocks.insert(current_blockhash);

        // we have to compute these variables before we consume the block with
        // `co.yield_`
        current_blockhash = current_block.previous_block_hash();
        let should_stop_here = should_stop_here(&current_block);

        if should_yield(&current_block.block_hash()) {
            co.yield_(current_block).await;
        }

        if should_stop_here {
            return Ok(seen_blocks);
        }

        if delay_until_fetch_latest_block_again.is_elapsed() {
            let latest_block = connector.latest_block().await?;
            let latest_block_hash = latest_block.block_hash();

            if !seen_blocks.contains(&latest_block_hash) && should_yield(&latest_block_hash) {
                seen_blocks.insert(latest_block_hash);
                co.yield_(latest_block).await;
            }

            delay_until_fetch_latest_block_again = tokio::time::delay_for(poll_interval)
        }

        current_block = connector.block_by_hash(current_blockhash).await?
    }
}

/// Constructs a predicate that returns `true` if the given block predates the
/// start_of_swap timestamp.
fn predates_start_of_swap<B>(start_of_swap: OffsetDateTime) -> impl Fn(&B) -> bool
where
    B: Predates,
{
    move |block| block.predates(start_of_swap)
}

/// Constructs a predicate that returns `true` if we have seen the given block
/// or the block predates the start_of_swap timestamp.
fn seen_block_or_predates_start_of_swap<'sb, B, H>(
    seen_blocks: &'sb HashSet<H>,
    start_of_swap: OffsetDateTime,
) -> impl Fn(&B) -> bool + 'sb
where
    B: Predates + BlockHash<BlockHash = H>,
    H: Eq + Hash,
{
    move |block: &B| {
        let have_seen_block = seen_blocks.contains(&block.block_hash());
        let predates_start_of_swap = predates_start_of_swap(start_of_swap)(block);

        have_seen_block || predates_start_of_swap
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{future, Stream, TryStreamExt};
    use genawaiter::GeneratorState;
    use tokio::{
        sync::Mutex,
        time::{delay_for, Delay},
    };

    /// A rough estimate of the IO latency for Infura.
    const INFURA_LATENCY: Duration = Duration::from_secs(4);
    /// The approximate mining speed of the Ethereum mainnet.
    const ETHEREUM_MAINNET_MINING_SPEED: Duration = Duration::from_secs(17);
    /// We use a poll interval of zero for the test because there is no need to
    /// artificially slow the processing down.
    const ZERO_POLL_INTERVAL: Duration = Duration::from_secs(0);

    #[tokio::test]
    async fn newly_mined_blocks_are_processed_within_a_short_amount_of_time() {
        let blocks = make_blockchain(100, ETHEREUM_MAINNET_MINING_SPEED);
        let start_of_swap = blocks[20].timestamp;
        let connector =
            FakeConnector::new(blocks, 50, INFURA_LATENCY, ETHEREUM_MAINNET_MINING_SPEED);

        let gen = fetch_blocks_since(&connector, start_of_swap, ZERO_POLL_INTERVAL);
        let yielded_blocks = fallible_generator_to_try_stream(gen)
            .map_ok(|b| b.number)
            .try_take_while(|n| future::ready(Ok(*n != 52)))
            .try_collect::<Vec<_>>();

        let processed_blocks = tokio::time::timeout(Duration::from_secs(60), yielded_blocks)
            .await
            .expect("block 52 to be yielded within 60 seconds")
            .expect("block processing to not fail");

        println!("{:?}", processed_blocks);
    }

    fn fallible_generator_to_try_stream<I, E, F: Future<Output = Result<Never, E>>>(
        gen: Gen<I, (), F>,
    ) -> impl Stream<Item = Result<I, E>> {
        futures::stream::try_unfold(gen, |mut gen| async move {
            Ok(match gen.async_resume().await {
                GeneratorState::Yielded(item) => Some((item, gen)),
                GeneratorState::Complete(Ok(never)) => match never {},
                GeneratorState::Complete(Err(e)) => return Err(e),
            })
        })
    }

    /// A connector for a fake blockchain.
    ///
    /// This connector stores a set of blocks together with a specific starting
    /// point. Additionally, we have an artificial latency whilst fetching
    /// blocks as well as a mining interval that moves the pointer to the
    /// current latest block forward.
    #[derive(Debug)]
    struct FakeConnector {
        blocks: Vec<FakeBlock>,
        io_latency: Duration,
        current_block: Mutex<usize>,
        mining_speed: Duration,
        time_until_next_block: Mutex<Delay>,
    }

    impl FakeConnector {
        fn new(
            blocks: Vec<FakeBlock>,
            current_block: usize,
            io_latency: Duration,
            mining_speed: Duration,
        ) -> Self {
            Self {
                blocks,
                io_latency,
                current_block: Mutex::new(current_block),
                mining_speed,
                time_until_next_block: Mutex::new(delay_for(mining_speed)),
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct FakeBlock {
        number: usize,
        timestamp: OffsetDateTime,
    }

    impl BlockHash for FakeBlock {
        type BlockHash = usize;

        fn block_hash(&self) -> Self::BlockHash {
            self.number
        }
    }

    impl PreviousBlockHash for FakeBlock {
        type BlockHash = usize;

        fn previous_block_hash(&self) -> Self::BlockHash {
            self.number - 1
        }
    }

    impl Predates for FakeBlock {
        fn predates(&self, timestamp: OffsetDateTime) -> bool {
            self.timestamp < timestamp
        }
    }

    #[async_trait]
    impl BlockByHash for FakeConnector {
        type Block = FakeBlock;
        type BlockHash = usize;

        async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block> {
            tokio::time::delay_for(self.io_latency).await;

            Ok(self.blocks[block_hash])
        }
    }

    #[async_trait]
    impl LatestBlock for FakeConnector {
        type Block = FakeBlock;

        async fn latest_block(&self) -> Result<Self::Block> {
            tokio::time::delay_for(self.io_latency).await;

            let mut delay = self.time_until_next_block.lock().await;
            let mut current_block = self.current_block.lock().await;

            let latest_block_index = if delay.is_elapsed() {
                *delay = delay_for(self.mining_speed);
                *current_block += 1;

                *current_block
            } else {
                *current_block
            };

            Ok(self.blocks[latest_block_index])
        }
    }

    /// Creates a blockchain of the specified size and approximate mining speed.
    ///
    /// The mining speed determines the timestamp of each block.
    fn make_blockchain(size: usize, mining_speed: Duration) -> Vec<FakeBlock> {
        let genesis = OffsetDateTime::from_unix_timestamp(1_000_000_000);

        (0..size)
            .map(|number| {
                #[allow(clippy::cast_possible_truncation)]
                let timestamp = genesis + mining_speed * (number as u32);

                FakeBlock { number, timestamp }
            })
            .collect()
    }
}
