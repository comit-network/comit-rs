mod cache;
mod web3_connector;

pub use self::{cache::Cache, web3_connector::Web3Connector};
use crate::{
    btsieve::{BlockByHash, LatestBlock, Predates, ReceiptByHash},
    ethereum::{Address, Bytes, IsStatusOk, Log, Transaction, TransactionReceipt, H256, U256, Input},
    Never,
};
use anyhow;
use chrono::NaiveDateTime;
use futures_core::compat::Future01CompatExt;
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::collections::HashSet;

type Hash = H256;
type Block = crate::ethereum::Block;

pub async fn watch_for_contract_creation<C>(
    blockchain_connector: C,
    start_of_swap: NaiveDateTime,
    bytecode: Bytes,
) -> anyhow::Result<(Transaction, Address)>
where
    C: LatestBlock<Block = Option<Block>>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>
        + Clone,
{
    let (transaction, receipt) =
        matching_transaction_and_receipt(blockchain_connector, start_of_swap, |transaction| {
            // transaction.to address is None if, and only if, the transaction
            // creates a contract.
            transaction.to.is_none() && transaction.input == bytecode
        })
        .await?;

    match receipt.contract_address {
        Some(location) => Ok((transaction, location)),
        None => Err(anyhow::anyhow!("contract address missing from receipt")),
    }
}

pub async fn watch_for_event<C>(
    blockchain_connector: C,
    start_of_swap: NaiveDateTime,
    event: Event,
) -> anyhow::Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Option<Block>>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>
        + Clone,
{
    matching_transaction_and_log(
        blockchain_connector.clone(),
        start_of_swap,
        event.topics.clone(),
        |receipt| find_log_for_event_in_receipt(&event, receipt),
    )
    .await
}

/// Fetch receipt from connector using transaction hash.
async fn fetch_receipt<C>(blockchain_connector: C, hash: Hash) -> anyhow::Result<TransactionReceipt>
where
    C: ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>,
{
    let receipt = blockchain_connector
        .receipt_by_hash(hash)
        .compat()
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not get transaction receipt for transaction {:x}",
                hash
            )
        })?;
    Ok(receipt)
}

fn find_log_for_event_in_receipt(event: &Event, receipt: TransactionReceipt) -> Option<Log> {
    match event {
        Event { topics, .. } if topics.is_empty() => None,
        Event { address, topics } => receipt.logs.into_iter().find(|log| {
            if address != &log.address {
                return false;
            }

            if log.topics.len() != topics.len() {
                return false;
            }

            log.topics.iter().enumerate().all(|(index, tx_topic)| {
                let topic = &topics[index];
                topic.as_ref().map_or(true, |topic| tx_topic == &topic.0)
            })
        }),
    }
}

pub async fn matching_transaction_and_receipt<C, F>(
    connector: C,
    start_of_swap: NaiveDateTime,
    matcher: F,
) -> anyhow::Result<(Transaction, TransactionReceipt)>
where
    C: LatestBlock<Block = Option<Block>>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>
        + Clone,
    F: Fn(Transaction) -> bool,
{
    let mut block_generator = Gen::new({
        let connector = connector.clone();
        |co| async move { find_relevant_blocks(connector, &co, start_of_swap).await }
    });

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                for transaction in block.transactions.into_iter() {
                    if matcher(transaction.clone()) {
                        let receipt = fetch_receipt(connector.clone(), transaction.hash).await?;
                        if !receipt.is_status_ok() {
                            // This can be caused by a failed attempt to complete an action,
                            // for example, sending a transaction with low gas.
                            tracing::warn!(
                                "transaction matched {:x} but status was NOT OK",
                                transaction.hash,
                            );
                            continue;
                        }
                        tracing::trace!("transaction matched {:x}", transaction.hash,);
                        return Ok((transaction, receipt));
                    }
                }
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this line was
            // written. The never type can never be constructed, so we can never reach this line.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
}

async fn matching_transaction_and_log<C, F>(
    connector: C,
    start_of_swap: NaiveDateTime,
    topics: Vec<Option<Topic>>,
    matcher: F,
) -> anyhow::Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Option<Block>>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>
        + Clone,
    F: Fn(TransactionReceipt) -> Option<Log>,
{
    let mut block_generator = Gen::new({
        let connector = connector.clone();
        |co| async move { find_relevant_blocks(connector, &co, start_of_swap).await }
    });

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                let block_hash = block
                    .hash
                    .ok_or_else(|| anyhow::anyhow!("block without hash"))?;

                let maybe_contains_transaction = topics.iter().all(|topic| {
                    topic.as_ref().map_or(true, |topic| {
                        block
                            .logs_bloom
                            .contains_input(Input::Raw(topic.0.as_ref()))
                    })
                });
                if !maybe_contains_transaction {
                    tracing::trace!(
                        "bloom filter indicates that block does not contain transaction:
                {:x}",
                        block_hash,
                    );
                    continue;
                }

                tracing::trace!(
                    "bloom filter indicates that we should check the block for transactions: {:x}",
                    block_hash,
                );
                for transaction in block.transactions.into_iter() {
                    let receipt = fetch_receipt(connector.clone(), transaction.hash).await?;
                    let status_is_ok = receipt.is_status_ok();
                    if let Some(log) = matcher(receipt) {
                        if !status_is_ok {
                            // This can be caused by a failed attempt to complete an action,
                            // for example, sending a transaction with low gas.
                            tracing::warn!(
                                "transaction matched {:x} but status was NOT OK",
                                transaction.hash,
                            );
                            continue;
                        }
                        tracing::trace!("transaction matched {:x}", transaction.hash,);
                        return Ok((transaction, log));
                    }
                }
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this line was
            // written. The never type can never be constructed, so we can never reach this line.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
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
async fn find_relevant_blocks<C>(
    mut connector: C,
    co: &Co<Block>,
    start_of_swap: NaiveDateTime,
) -> anyhow::Result<Never>
where
    C: LatestBlock<Block = Option<Block>>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>
        + Clone,
{
    let mut seen_blocks: HashSet<Hash> = HashSet::new();

    let block = connector
        .latest_block()
        .compat()
        .await?
        .ok_or_else(|| anyhow::anyhow!("Connector returned null latest block"))?;
    co.yield_(block.clone()).await;

    let blockhash = block
        .hash
        .ok_or_else(|| anyhow::anyhow!("Connector returned latest block with null hash"))?;
    seen_blocks.insert(blockhash);

    // Look back in time until we get a block that predates start_of_swap.
    let parent_hash = block.parent_hash;
    walk_back_until(
        predates_start_of_swap(start_of_swap),
        connector.clone(),
        co,
        parent_hash,
    )
    .await?;

    loop {
        let block = connector
            .latest_block()
            .compat()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Connector returned null latest block"))?;
        co.yield_(block.clone()).await;

        let blockhash = block
            .hash
            .ok_or_else(|| anyhow::anyhow!("Connector returned latest block with null hash"))?;
        seen_blocks.insert(blockhash);

        // Look back along the blockchain for missing blocks.
        let parent_hash = block.parent_hash;
        if !seen_blocks.contains(&parent_hash) {
            walk_back_until(
                seen_block_or_predates_start_of_swap(seen_blocks.clone(), start_of_swap),
                connector.clone(),
                co,
                parent_hash,
            )
            .await?;
        }

        // The duration of this timeout could/should depend on the network
        tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    }
}

/// Walks the blockchain backwards from the given hash until the predicate given
/// in `stop_condition` returns `true`.
///
/// This functions yields all blocks as part of its process.
async fn walk_back_until<C, P>(
    stop_condition: P,
    connector: C,
    co: &Co<Block>,
    starting_blockhash: Hash,
) -> anyhow::Result<()>
where
    C: BlockByHash<Block = Option<Block>, BlockHash = Hash>,
    P: Fn(&Block) -> anyhow::Result<bool>,
{
    let mut blockhash = starting_blockhash;

    loop {
        let block = connector
            .block_by_hash(blockhash)
            .compat()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Could not fetch block with hash {}", blockhash))?;

        co.yield_(block.clone()).await;

        if stop_condition(&block)? {
            return Ok(());
        } else {
            blockhash = block.parent_hash
        }
    }
}

/// Constructs a predicate that returns `true` if the given block predates the
/// start_of_swap timestamp.
fn predates_start_of_swap(start_of_swap: NaiveDateTime) -> impl Fn(&Block) -> anyhow::Result<bool> {
    move |block| Ok(block.predates(start_of_swap))
}

/// Constructs a predicate that returns `true` if we have seen the given block
/// or the block predates the start_of_swap timestamp.
fn seen_block_or_predates_start_of_swap(
    seen_blocks: HashSet<Hash>,
    start_of_swap: NaiveDateTime,
) -> impl Fn(&Block) -> anyhow::Result<bool> {
    move |block: &Block| {
        let have_seen_block = seen_blocks.contains(
            &block
                .hash
                .ok_or_else(|| anyhow::anyhow!("block without hash"))?,
        );
        let predates_start_of_swap = predates_start_of_swap(start_of_swap)(block)?;

        Ok(have_seen_block || predates_start_of_swap)
    }
}

impl Predates for Block {
    fn predates(&self, timestamp: NaiveDateTime) -> bool {
        let unix_timestamp = timestamp.timestamp();

        self.timestamp < U256::from(unix_timestamp)
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq, serde::Serialize, serdebug::SerDebug)]
#[serde(transparent)]
pub struct Topic(pub H256);

/// Event works similar to web3 filters:
/// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
/// For example, this `Event` would match this `Log`:
/// ```rust, ignore
/// 
/// Event {
/// 	address: "0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59",
/// 	topics: [
/// 	    None,
/// 	    Some("0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59"),
///         None,
///     ],
/// }
///
/// Log: {
/// 	address: "0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59",
/// 	data: "0x123",
/// 	topics: [
/// 	    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
/// 	    "0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59",
/// 	    "0x000000000000000000000000d51ecee7414c4445534f74208538683702cbb3e4",
/// 	]
/// 	...  // Other data omitted
/// }
/// ```
#[derive(Clone, Default, Eq, PartialEq, serde::Serialize, serdebug::SerDebug)]
pub struct Event {
    pub address: Address,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<Option<Topic>>,
}
