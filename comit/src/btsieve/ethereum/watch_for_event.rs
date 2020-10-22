use crate::{
    btsieve::{
        ethereum::{poll_interval, Event, GetLogs, ReceiptByHash, TransactionByHash},
        BlockByHash, ConnectedNetwork, LatestBlock,
    },
    ethereum::{Block, ChainId, Hash, Log, Transaction},
};
use anyhow::Result;
use time::OffsetDateTime;

pub async fn watch_for_event<C>(
    connector: &C,
    _start_of_swap: OffsetDateTime,
    expected_event: Event,
) -> Result<(Transaction, Log)>
where
    C: LatestBlock<Block = Block>
        + BlockByHash<Block = Block, BlockHash = Hash>
        + ReceiptByHash
        + TransactionByHash
        + ConnectedNetwork<Network = ChainId>
        + GetLogs,
{
    let poll_interval = poll_interval(connector).await?;

    loop {
        let logs = connector.get_logs(expected_event.clone()).await?;

        if let Some(log) = find_log_for_event(&expected_event, logs) {
            let tx = connector.transaction_by_hash(log.transaction_hash).await?;

            return Ok((tx, log));
        }

        tokio::time::delay_for(poll_interval).await;
    }
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
                topic.as_ref().map_or(true, |topic| tx_topic == topic)
            })
        }),
    }
}
