pub mod bitcoin;
pub mod ethereum;
mod jsonrpc;

use crate::Never;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use genawaiter::sync::{Co, Gen};
use std::{collections::HashSet, future::Future, hash::Hash};

#[async_trait]
pub trait LatestBlock: Send + Sync + 'static {
    type Block;

    async fn latest_block(&self) -> anyhow::Result<Self::Block>;
}

#[async_trait]
pub trait BlockByHash: Send + Sync + 'static {
    type Block;
    type BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block>;
}

/// Checks if a given block predates a certain timestamp.
pub trait Predates {
    fn predates(&self, timestamp: DateTime<Utc>) -> bool;
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
    start_of_swap: DateTime<Utc>,
) -> Gen<B, (), impl Future<Output = anyhow::Result<Never>> + 'a>
where
    C: LatestBlock<Block = B> + BlockByHash<Block = B, BlockHash = H>,
    B: Predates + BlockHash<BlockHash = H> + PreviousBlockHash<BlockHash = H> + Clone + 'a,
    H: Eq + Hash + Copy,
{
    Gen::new(|co| async move {
        let block = connector.latest_block().await?;

        // Look back in time until we get a block that predates start_of_swap.
        let mut seen_blocks =
            walk_back_until(predates_start_of_swap(start_of_swap), block, connector, &co).await?;

        // Look forward in time, but keep going back for missed blocks
        loop {
            let block = connector.latest_block().await?;

            let missed_blocks = walk_back_until(
                seen_block_or_predates_start_of_swap(&seen_blocks, start_of_swap),
                block,
                connector,
                &co,
            )
            .await?;

            seen_blocks.extend(missed_blocks);

            // The duration of this timeout could/should depend on the network
            tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
        }
    })
}

/// Walks the blockchain backwards from the given hash until the predicate given
/// in `stop_condition` returns `true`.
///
/// This function yields all blocks as part of its process.
/// This function returns the block-hashes of all visited blocks.
async fn walk_back_until<C, P, B, H>(
    should_stop_here: P,
    starting_block: B,
    connector: &C,
    co: &Co<B>,
) -> anyhow::Result<HashSet<H>>
where
    C: BlockByHash<Block = B, BlockHash = H>,
    P: Fn(&B) -> bool,
    B: BlockHash<BlockHash = H> + PreviousBlockHash<BlockHash = H>,
    H: Eq + Hash + Copy,
{
    let mut seen_blocks = HashSet::new();

    let mut current_blockhash = starting_block.block_hash();
    let mut current_block = starting_block;

    loop {
        seen_blocks.insert(current_blockhash);

        // we have to compute these variables before we consume the block with
        // `co.yield_`
        current_blockhash = current_block.previous_block_hash();
        let should_stop_here = should_stop_here(&current_block);

        // we have to yield the block before exiting
        co.yield_(current_block).await;

        if should_stop_here {
            return Ok(seen_blocks);
        }

        current_block = connector.block_by_hash(current_blockhash).await?
    }
}

/// Constructs a predicate that returns `true` if the given block predates the
/// start_of_swap timestamp.
fn predates_start_of_swap<B>(start_of_swap: DateTime<Utc>) -> impl Fn(&B) -> bool
where
    B: Predates,
{
    move |block| block.predates(start_of_swap)
}

/// Constructs a predicate that returns `true` if we have seen the given block
/// or the block predates the start_of_swap timestamp.
fn seen_block_or_predates_start_of_swap<'sb, B, H>(
    seen_blocks: &'sb HashSet<H>,
    start_of_swap: DateTime<Utc>,
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
