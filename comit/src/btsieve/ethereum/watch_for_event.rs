use crate::{
    btsieve::{
        ethereum::{poll_interval, Event, ReceiptByHash, Topic},
        fetch_blocks_since, BlockByHash, ConnectedNetwork, LatestBlock,
    },
    ethereum::{Block, ChainId, Hash, Input, Log, Transaction, TransactionReceipt},
};
use anyhow::Result;
use genawaiter::GeneratorState;
use time::OffsetDateTime;
use tracing_futures::Instrument;

pub async fn watch_for_event<C>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    expected_event: Event,
) -> Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + ConnectedNetwork<Network = ChainId>,
{
    matching_transaction_and_log(
        connector,
        start_of_swap,
        expected_event.topics.clone(),
        |receipt| find_log_for_event(&expected_event, receipt.logs),
    )
    .await
}

fn find_log_for_event(event: &Event, logs: Vec<Log>) -> Option<Log> {
    match event {
        Event { topics, .. } if topics.is_empty() => None,
        Event { address, topics } => logs.into_iter().find(|log| {
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

async fn matching_transaction_and_log<C, F>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    topics: Vec<Option<Topic>>,
    matcher: F,
) -> Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + ConnectedNetwork<Network = ChainId>,
    F: Fn(TransactionReceipt) -> Option<Log> + Clone,
{
    let poll_interval = poll_interval(connector).await?;
    let mut block_generator = fetch_blocks_since(connector, start_of_swap, poll_interval);

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                if let Some(result) =
                    process_block(block, connector, topics.clone(), matcher.clone()).await?
                {
                    return Ok(result);
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

#[tracing::instrument(name = "block", skip(block, connector, matcher, topics), fields(hash = %block.hash, tx_count = %block.transactions.len()))]
async fn process_block<C, F>(
    block: Block,
    connector: &C,
    topics: Vec<Option<Topic>>,
    matcher: F,
) -> Result<Option<(Transaction, Log)>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(TransactionReceipt) -> Option<Log> + Clone,
{
    let maybe_contains_transaction = topics.iter().all(|topic| {
        topic.as_ref().map_or(true, |topic| {
            block
                .logs_bloom
                .contains_input(Input::Raw(&topic.0.as_bytes()))
        })
    });

    if !maybe_contains_transaction {
        tracing::trace!("skipping block due to bloom filter");
        return Ok(None);
    }

    for transaction in block.transactions.into_iter() {
        if let Some(result) = process_transaction(transaction, connector, matcher.clone())
            .in_current_span()
            .await?
        {
            return Ok(Some(result));
        }
    }

    tracing::debug!("no transaction matched");

    Ok(None)
}

#[tracing::instrument(name = "tx", skip(tx, connector, matcher), fields(hash = %tx.hash))]
async fn process_transaction<C, F>(
    tx: Transaction,
    connector: &C,
    matcher: F,
) -> Result<Option<(Transaction, Log)>>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(TransactionReceipt) -> Option<Log>,
{
    let receipt = connector.receipt_by_hash(tx.hash).await?;
    let is_successful = receipt.successful;

    if let Some(log) = matcher(receipt) {
        if !is_successful {
            // This can be caused by a failed attempt to complete an action,
            // for example, sending a transaction with low gas.
            tracing::warn!("transaction matched but status was NOT OK");
            return Ok(None);
        }
        tracing::info!("transaction matched");

        return Ok(Some((tx, log)));
    }

    Ok(None)
}
