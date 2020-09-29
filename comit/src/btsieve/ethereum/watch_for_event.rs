use crate::{
    btsieve::{
        ethereum::{Event, ReceiptByHash, Topic},
        fetch_blocks_since, BlockByHash, LatestBlock,
    },
    ethereum::{Block, Hash, Input, Log, Transaction, TransactionReceipt},
};
use anyhow::Result;
use genawaiter::GeneratorState;
use time::OffsetDateTime;

// This tracing context is useful because it conveys information through its
// name although we skip all fields because they would add too much noise.
#[tracing::instrument(level = "debug", skip(connector, start_of_swap, expected_event))]
pub async fn watch_for_event<C>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    expected_event: Event,
) -> Result<(Transaction, Log)>
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

async fn matching_transaction_and_log<C, F>(
    connector: &C,
    start_of_swap: OffsetDateTime,
    topics: Vec<Option<Topic>>,
    matcher: F,
) -> Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Block> + BlockByHash<Block = Block, BlockHash = Hash> + ReceiptByHash,
    F: Fn(TransactionReceipt) -> Option<Log>,
{
    let mut block_generator = fetch_blocks_since(connector, start_of_swap);

    loop {
        match block_generator.async_resume().await {
            GeneratorState::Yielded(block) => {
                let block_span = tracing::error_span!("block", hash = %block.hash, tx_count = %block.transactions.len());
                let _enter_block_span = block_span.enter();

                let maybe_contains_transaction = topics.iter().all(|topic| {
                    topic.as_ref().map_or(true, |topic| {
                        block
                            .logs_bloom
                            .contains_input(Input::Raw(&topic.0.as_bytes()))
                    })
                });
                if !maybe_contains_transaction {
                    tracing::trace!("skipping block due to bloom filter");
                    continue;
                }

                for transaction in block.transactions.into_iter() {
                    let receipt = connector.receipt_by_hash(transaction.hash).await?;
                    let is_successful = receipt.successful;

                    let tx_span = tracing::error_span!("tx", hash = %transaction.hash);
                    let _enter_tx_span = tx_span.enter();

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

                tracing::debug!("no transaction matched")
            }
            GeneratorState::Complete(Err(e)) => return Err(e),
            // By matching against the never type explicitly, we assert that the `Ok` value of the
            // result is actually the never type and has not been changed since this line was
            // written. The never type can never be constructed, so we can never reach this line.
            GeneratorState::Complete(Ok(never)) => match never {},
        }
    }
}
