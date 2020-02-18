mod cache;
mod web3_connector;

pub use self::{cache::Cache, web3_connector::Web3Connector};
use crate::{
    btsieve::{BlockByHash, LatestBlock, Predates, ReceiptByHash},
    ethereum::{Address, Bytes, Log, Transaction, TransactionReceipt, H256, U256},
    Never,
};
use anyhow;
use chrono::NaiveDateTime;
use ethbloom::Input;
use futures_core::compat::Future01CompatExt;
use genawaiter::{
    sync::{Co, Gen},
    GeneratorState,
};
use std::collections::HashSet;

type Hash = H256;
type Block = crate::ethereum::Block<Transaction>;

pub const TRANSACTION_STATUS_OK: u32 = 1;

pub async fn matching_create_contract<C>(
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

pub async fn matching_event<C>(
    blockchain_connector: C,
    start_of_swap: NaiveDateTime,
    event: Event,
    action: &'static str,
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
        action,
        |receipt| {
            if event_exists_in_receipt(&event, &receipt) {
                let log_msg = &event.topics[0].unwrap().0;
                let log = receipt
                    .logs
                    .into_iter()
                    .find(|log| log.topics.contains(log_msg))
                    .ok_or_else(|| {
                        anyhow::anyhow!("Fund transaction receipt must contain transfer event")
                    })?;

                return Ok(Some(log));
            }

            Ok(None)
        },
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

fn event_exists_in_receipt(event: &Event, receipt: &TransactionReceipt) -> bool {
    match event {
        Event { topics, .. } if topics.is_empty() => false,
        Event { address, topics } => receipt.logs.iter().any(|tx_log| {
            if address != &tx_log.address {
                return false;
            }

            if tx_log.topics.len() == topics.len() {
                tx_log.topics.iter().enumerate().all(|(index, tx_topic)| {
                    let topic = &topics[index];
                    topic.as_ref().map_or(true, |topic| tx_topic == &topic.0)
                })
            } else {
                false
            }
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
                        if !receipt.transaction_status_ok() {
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
            // result is actually the never type and has not been changed since this
            // line was written. The never type can never be constructed, so we cannot
            // reach this line never anyway.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
}

async fn matching_transaction_and_log<C, F>(
    connector: C,
    start_of_swap: NaiveDateTime,
    topics: Vec<Option<Topic>>,
    action: &str,
    matcher: F,
) -> anyhow::Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Option<Block>>
        + BlockByHash<Block = Option<Block>, BlockHash = Hash>
        + ReceiptByHash<Receipt = Option<TransactionReceipt>, TransactionHash = Hash>
        + Clone,
    F: Fn(TransactionReceipt) -> anyhow::Result<Option<Log>>,
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
                        "bloom filter indicates that block does not contain {} transaction:
                {:x}",
                        action,
                        block_hash,
                    );
                    continue;
                }

                tracing::trace!(
                    "bloom filter indicates that we should check the block for {} transactions: {:x}",
                    action,
                    block_hash,
                );
                for transaction in block.transactions.into_iter() {
                    let receipt = fetch_receipt(connector.clone(), transaction.hash).await?;
                    if let Some(log) = matcher(receipt.clone())? {
                        if !receipt.transaction_status_ok() {
                            // This can be caused by a failed attempt to complete an action,
                            // for example, sending a transaction with low gas.
                            tracing::warn!(
                                "{} transaction matched {:x} but status was NOT OK",
                                action,
                                transaction.hash,
                            );
                            continue;
                        }
                        tracing::trace!("{} transaction matched {:x}", action, transaction.hash,);
                        return Ok((transaction.clone(), log));
                    }
                }
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this
            // line was written. The never type can never be constructed, so we cannot
            // reach this line never anyway.
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
/// E.g. this `Event` would match this `Log`:
/// ```rust, ignore
/// Event {
/// address: "0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59",
/// topics: [
/// None,
/// 0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59,
/// None,
/// ],
/// ```
/// ```rust, ignore
/// Log:
/// [ { address: "0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59",
/// data: "0x123",
/// ..
/// topics:
/// [ "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef",
/// "0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59",
/// "0x000000000000000000000000d51ecee7414c4445534f74208538683702cbb3e4" ],
/// },
/// .. ] //Other data omitted
/// }
/// ```
#[derive(Clone, Default, Eq, PartialEq, serde::Serialize, serdebug::SerDebug)]
pub struct Event {
    pub address: Address,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<Option<Topic>>,
}

trait TransactionStatusOk {
    fn transaction_status_ok(&self) -> bool;
}

impl TransactionStatusOk for TransactionReceipt {
    fn transaction_status_ok(&self) -> bool {
        const TRANSACTION_STATUS_OK: u32 = 1;
        self.status == Some(TRANSACTION_STATUS_OK.into())
    }
}
