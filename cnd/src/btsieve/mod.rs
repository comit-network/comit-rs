#![warn(rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod bitcoin;
pub mod ethereum;

use crate::Never;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use genawaiter::sync::Co;
use std::{collections::HashSet, hash::Hash};

#[async_trait]
pub trait LatestBlock: Send + Sync + 'static {
    type Block;
    type BlockHash;

    async fn latest_block(&mut self) -> anyhow::Result<Self::Block>;
}

#[async_trait]
pub trait BlockByHash: Send + Sync + 'static {
    type Block;
    type BlockHash;

    async fn block_by_hash(&mut self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block>;
}

/// Checks if a given block predates a certain timestamp.
pub trait Predates {
    fn predates(&self, timestamp: NaiveDateTime) -> bool;
}

pub trait BlockHash<H> {
    fn block_hash(&self) -> H;
}

pub trait PreviousBlockHash<H> {
    fn previous_block_hash(&self) -> H;
}

/// This function uses the `connector` to find blocks relevant to a swap.  To do
/// this we must get the latest block, for each latest block we receive we must
/// ensure that we saw its parent i.e., that we did not miss any blocks between
/// this latest block and the previous latest block we received.  Finally, we
/// must also get each block back until the time that the swap started i.e.,
/// look into the past (in case any action occurred on chain while we were not
/// watching).
///
/// It yields those blocks as part of the process.
pub async fn find_relevant_blocks<C, B, H>(
    mut connector: C,
    co: &Co<B>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Never>
where
    C: LatestBlock<Block = B> + BlockByHash<Block = B, BlockHash = H> + Clone,
    B: Predates + BlockHash<H> + PreviousBlockHash<H> + Clone,
    H: Eq + Hash + Copy,
{
    let block = connector.latest_block().await?;

    // Look back in time until we get a block that predates start_of_swap.
    let mut seen_blocks = walk_back_until(
        predates_start_of_swap(start_of_swap),
        connector.clone(),
        co,
        block,
    )
    .await?;

    // Look forward in time, but keep going back for missed blocks
    loop {
        let block = connector.latest_block().await?;

        let missed_blocks = walk_back_until(
            seen_block_or_predates_start_of_swap(&seen_blocks, start_of_swap),
            connector.clone(),
            co,
            block,
        )
        .await?;

        seen_blocks.extend(missed_blocks);

        // The duration of this timeout could/should depend on the network
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    }
}

/// Walks the blockchain backwards from the given hash until the predicate given
/// in `stop_condition` returns `true`.
///
/// This function yields all blocks as part of its process.
/// This function returns the block-hashes of all visited blocks.
async fn walk_back_until<C, P, B, H>(
    stop_condition: P,
    connector: C,
    co: &Co<B>,
    block: B,
) -> anyhow::Result<HashSet<H>>
where
    C: BlockByHash<Block = B, BlockHash = H> + Clone,
    P: Fn(&B) -> anyhow::Result<bool>,
    B: BlockHash<H> + PreviousBlockHash<H> + Clone,
    H: Eq + Hash + Copy,
{
    let mut seen_blocks: HashSet<H> = HashSet::new();
    let mut blockhash = block.block_hash();
    let mut connector = connector.clone();

    co.yield_(block.clone()).await;
    seen_blocks.insert(blockhash);

    if stop_condition(&block)? {
        return Ok(seen_blocks);
    } else {
        blockhash = block.previous_block_hash();
    }

    loop {
        let block = connector.block_by_hash(blockhash).await?;
        co.yield_(block.clone()).await;
        seen_blocks.insert(blockhash);

        if stop_condition(&block)? {
            return Ok(seen_blocks);
        } else {
            blockhash = block.previous_block_hash();
        }
    }
}

/// Constructs a predicate that returns `true` if the given block predates the
/// start_of_swap timestamp.
fn predates_start_of_swap<B>(start_of_swap: NaiveDateTime) -> impl Fn(&B) -> anyhow::Result<bool>
where
    B: Predates,
{
    move |block| Ok(block.predates(start_of_swap))
}

/// Constructs a predicate that returns `true` if we have seen the given block
/// or the block predates the start_of_swap timestamp.
fn seen_block_or_predates_start_of_swap<'sb, B, H>(
    seen_blocks: &'sb HashSet<H>,
    start_of_swap: NaiveDateTime,
) -> impl Fn(&B) -> anyhow::Result<bool> + 'sb
where
    B: Predates + BlockHash<H>,
    H: Eq + Hash,
{
    move |block: &B| {
        let have_seen_block = seen_blocks.contains(&block.block_hash());
        let predates_start_of_swap = predates_start_of_swap(start_of_swap)(block)?;

        Ok(have_seen_block || predates_start_of_swap)
    }
}
