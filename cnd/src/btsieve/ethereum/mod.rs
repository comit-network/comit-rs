mod transaction_pattern;
mod web3_connector;

pub use self::{
    transaction_pattern::{Event, Topic, TransactionPattern},
    web3_connector::Web3Connector,
};
use crate::{
    btsieve::{BlockByHash, LatestBlock, ReceiptByHash},
    ethereum::{Transaction, TransactionAndReceipt, TransactionReceipt, H256, U256},
};
use anyhow;
use futures_core::compat::Future01CompatExt;
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::{collections::HashSet, fmt::Debug};

type Hash = H256;
type Block = crate::ethereum::Block<Transaction>;

pub async fn matching_transaction<C, E>(
    connector: C,
    pattern: TransactionPattern,
    reference_timestamp: Option<u32>,
) -> anyhow::Result<TransactionAndReceipt>
where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: std::error::Error + Debug + Send + 'static + Sync,
{
    let mut block_generator = Gen::new({
        let connector = connector.clone();
        |co| async move { yield_blocks(connector, &co, reference_timestamp).await }
    });

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                if let Some(transaction_and_receipt) =
                    check_block_against_pattern(connector.clone(), block, pattern.clone()).await?
                {
                    return Ok(transaction_and_receipt);
                } else {
                    continue;
                }
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            GeneratorState::Complete(Ok(infallible)) => match infallible {},
        }
    }
}

async fn yield_blocks<C, E>(
    mut connector: C,
    co: &Co<Block>,
    reference_timestamp: Option<u32>,
) -> anyhow::Result<std::convert::Infallible>
where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: std::error::Error + Debug + Send + Sync + 'static,
{
    let mut seen_blockhashes: HashSet<Hash> = HashSet::new();

    loop {
        // The duration of this timeout could/should depend on the network
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;

        let block = connector
            .latest_block()
            .compat()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connector returned nullable latest block"))?;

        let blockhash = block
            .hash
            .ok_or_else(|| anyhow::anyhow!("Connector returned latest block with nullable hash"))?;
        seen_blockhashes.insert(blockhash);

        if let Some(timestamp) = reference_timestamp {
            if seen_blockhashes.len() == 1 && block.timestamp > U256::from(timestamp) {
                yield_blocks_until_timestamp(connector.clone(), co, blockhash, timestamp).await?;
            }
        }

        let parent_hash = block.parent_hash;
        if !seen_blockhashes.contains(&parent_hash) && seen_blockhashes.len() > 1 {
            yield_missed_blocks(
                connector.clone(),
                co,
                parent_hash,
                seen_blockhashes.clone(),
                reference_timestamp.unwrap_or(0),
            )
            .await?;
        }

        co.yield_(block).await;
    }
}

async fn yield_blocks_until_timestamp<C, E>(
    connector: C,
    co: &Co<Block>,
    starting_blockhash: Hash,
    timestamp: u32,
) -> anyhow::Result<()>
where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: std::error::Error + Debug + Send + Sync + 'static,
{
    let mut blockhash = starting_blockhash;

    loop {
        let block = connector
            .block_by_hash(blockhash)
            .compat()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not fetch block with hash {}", blockhash))?;

        co.yield_(block.clone()).await;

        if block.timestamp <= U256::from(timestamp) {
            return Ok(());
        } else {
            blockhash = block.parent_hash
        }
    }
}

async fn yield_missed_blocks<C, E>(
    connector: C,
    co: &Co<Block>,
    starting_blockhash: Hash,
    seen_blockhashes: HashSet<Hash>,
    timestamp: u32,
) -> anyhow::Result<()>
where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: std::error::Error + Debug + Send + Sync + 'static,
{
    let mut blockhash = starting_blockhash;

    loop {
        let block = connector
            .block_by_hash(blockhash)
            .compat()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not fetch block with hash {}", blockhash))?;

        co.yield_(block.clone()).await;

        if seen_blockhashes.contains(
            &block
                .hash
                .ok_or_else(|| anyhow::anyhow!("Block with nullable hash"))?,
        ) || U256::from(timestamp) >= block.timestamp
        {
            return Ok(());
        } else {
            blockhash = block.parent_hash
        }
    }
}

async fn check_block_against_pattern<C, E>(
    connector: C,
    block: Block,
    pattern: TransactionPattern,
) -> anyhow::Result<Option<TransactionAndReceipt>>
where
    C: LatestBlock<Block = Option<Block>, Error = E>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash, Error = E>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash, Error = E>
        + Clone,
    E: std::error::Error + Debug + Send + Sync + 'static,
{
    let needs_receipt = pattern.needs_receipts(&block);

    for transaction in block.transactions.into_iter() {
        if needs_receipt {
            let hash = transaction.hash;
            let receipt = connector
                .receipt_by_hash(hash)
                .compat()
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("Could not get transaction receipt for transaction {}", hash)
                })?;

            if pattern.matches(&transaction, Some(&receipt)) {
                return Ok(Some(TransactionAndReceipt {
                    transaction,
                    receipt,
                }));
            }
        } else if pattern.matches(&transaction, None) {
            let hash = transaction.hash;
            let receipt = connector
                .receipt_by_hash(hash)
                .compat()
                .await?
                .ok_or_else(|| {
                    anyhow::anyhow!("Could not get transaction receipt for transaction {}", hash)
                })?;

            return Ok(Some(TransactionAndReceipt {
                transaction,
                receipt,
            }));
        }
    }

    Ok(None)
}
