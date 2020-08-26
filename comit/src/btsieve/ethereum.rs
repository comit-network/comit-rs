mod cache;
mod web3_connector;

pub use self::{cache::Cache, web3_connector::Web3Connector};
use crate::{
    btsieve::{
        fetch_blocks_since, BlockByHash, BlockHash, LatestBlock, Predates, PreviousBlockHash,
    },
    ethereum::{Address, Block, Hash, Input, Log, Transaction, TransactionReceipt, U256},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use genawaiter::GeneratorState;

#[async_trait]
pub trait ReceiptByHash: Send + Sync + 'static {
    async fn receipt_by_hash(&self, transaction_hash: Hash) -> anyhow::Result<TransactionReceipt>;
}

impl BlockHash for Block {
    type BlockHash = Hash;

    fn block_hash(&self) -> Hash {
        self.hash
    }
}

impl PreviousBlockHash for Block {
    type BlockHash = Hash;

    fn previous_block_hash(&self) -> Hash {
        self.parent_hash
    }
}

// This tracing context is useful because it conveys information through its
// name although we skip all fields because they would add too much noise.
#[tracing::instrument(level = "debug", skip(connector, start_of_swap, expected_bytecode))]
pub async fn watch_for_contract_creation<C>(
    connector: &C,
    start_of_swap: DateTime<Utc>,
    expected_bytecode: &[u8],
) -> anyhow::Result<(Transaction, Address)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    let (transaction, receipt) =
        matching_transaction_and_receipt(connector, start_of_swap, |transaction| {
            // transaction.to address is None if, and only if, the transaction
            // creates a contract.

            let is_contract_creation = transaction.to.is_none();
            let is_expected_contract = transaction.input.as_slice() == expected_bytecode;

            if !is_contract_creation {
                tracing::trace!("rejected because transaction doesn't create a contract");
            }

            if !is_expected_contract {
                tracing::trace!("rejected because contract code doesn't match");

                // only compute levenshtein distance if we are on trace level, converting to hex is expensive at this scale
                if tracing::level_enabled!(tracing::level_filters::LevelFilter::TRACE) {
                    let actual = hex::encode(&transaction.input);
                    let expected = hex::encode(expected_bytecode);

                    let distance = levenshtein::levenshtein(&actual, &expected);

                    // We probably need to find a meaningful value here, expiry is 4 bytes.
                    if distance < 10 {
                        tracing::warn!("found contract with slightly different parameters (levenshtein-distance < 10), this could be a bug!")
                    }
                }
            }

            is_contract_creation && is_expected_contract
        })
        .await?;

    match receipt.contract_address {
        Some(location) => Ok((transaction, location)),
        None => Err(anyhow::anyhow!("contract address missing from receipt")),
    }
}

// This tracing context is useful because it conveys information through its
// name although we skip all fields because they would add too much noise.
#[tracing::instrument(level = "debug", skip(connector, start_of_swap, expected_event))]
pub async fn watch_for_event<C>(
    connector: &C,
    start_of_swap: DateTime<Utc>,
    expected_event: Event,
) -> anyhow::Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
{
    matching_transaction_and_log(
        connector,
        start_of_swap,
        expected_event.topics.clone(),
        |receipt| find_log_for_event_in_receipt(&expected_event, receipt),
    )
    .await
}

/// Fetch receipt from connector using transaction hash.
async fn fetch_receipt<C>(
    blockchain_connector: &C,
    hash: Hash,
) -> anyhow::Result<TransactionReceipt>
where
    C: ReceiptByHash,
{
    let receipt = blockchain_connector.receipt_by_hash(hash).await?;
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
    connector: &C,
    start_of_swap: DateTime<Utc>,
    matcher: F,
) -> anyhow::Result<(Transaction, TransactionReceipt)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(&Transaction) -> bool,
{
    let mut block_generator = fetch_blocks_since(connector, start_of_swap);

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                let span =
                    tracing::trace_span!("new_block", blockhash = format_args!("{}", block.hash));
                let _enter = span.enter();

                tracing::trace!("checking {} transactions", block.transactions.len());

                for transaction in block.transactions.into_iter() {
                    let tx_hash = transaction.hash;
                    let span = tracing::trace_span!(
                        "matching_transaction",
                        txhash = format_args!("{}", tx_hash)
                    );
                    let _enter = span.enter();

                    if matcher(&transaction) {
                        let receipt = fetch_receipt(connector, tx_hash).await?;
                        if !receipt.successful {
                            // This can be caused by a failed attempt to complete an action,
                            // for example, sending a transaction with low gas.
                            tracing::warn!("transaction matched but status was NOT OK");
                            continue;
                        }
                        tracing::info!("transaction matched");
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
    connector: &C,
    start_of_swap: DateTime<Utc>,
    topics: Vec<Option<Topic>>,
    matcher: F,
) -> anyhow::Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(TransactionReceipt) -> Option<Log>,
{
    let mut block_generator = fetch_blocks_since(connector, start_of_swap);

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                let span =
                    tracing::trace_span!("new_block", blockhash = format_args!("{}", block.hash));
                let _enter = span.enter();

                let maybe_contains_transaction = topics.iter().all(|topic| {
                    topic.as_ref().map_or(true, |topic| {
                        block
                            .logs_bloom
                            .contains_input(Input::Raw(&topic.0.as_bytes()))
                    })
                });
                if !maybe_contains_transaction {
                    tracing::trace!(
                    "bloom filter indicates that this block will not contain an instance of the event"

                    );
                    continue;
                } else {
                    tracing::trace!(
                        "bloom filter indicates that this block might contain an instance of the event"
                    );
                }

                tracing::trace!("checking {} transactions", block.transactions.len());

                for transaction in block.transactions.into_iter() {
                    let tx_hash = transaction.hash;

                    let span = tracing::trace_span!(
                        "matching_transaction",
                        txhash = format_args!("{}", tx_hash)
                    );
                    let _enter = span.enter();

                    let receipt = fetch_receipt(connector, tx_hash).await?;
                    let is_successful = receipt.successful;
                    if let Some(log) = matcher(receipt) {
                        if !is_successful {
                            // This can be caused by a failed attempt to complete an action,
                            // for example, sending a transaction with low gas.
                            tracing::warn!("transaction matched but status was NOT OK");
                            continue;
                        }
                        tracing::info!("transaction matched");
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

impl Predates for Block {
    fn predates(&self, timestamp: DateTime<Utc>) -> bool {
        let unix_timestamp = timestamp.timestamp();

        self.timestamp < U256::from(unix_timestamp)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Topic(pub Hash);

/// Event works similar to web3 filters:
/// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Event {
    pub address: Address,
    pub topics: Vec<Option<Topic>>,
}
