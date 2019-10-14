mod queries;
mod web3_connector;

#[cfg(test)]
mod quickcheck_impls;

pub use self::{
    queries::{EventMatcher, Topic, TransactionQuery},
    web3_connector::Web3Connector,
};
use crate::{BlockByHash, LatestBlock, MatchingTransactions, ReceiptByHash};
use ethereum_support::{
    web3::{
        futures::{Future, Stream},
        types::Block,
    },
    TransactionAndReceipt,
};
use std::time::Duration;
use tokio::timer::Interval;

impl<B> MatchingTransactions<TransactionQuery> for B
where
    B: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + BlockByHash<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + ReceiptByHash<
            Receipt = Option<ethereum_support::TransactionReceipt>,
            TransactionHash = ethereum_support::H256,
        > + Clone,
{
    type Transaction = TransactionAndReceipt;

    fn matching_transactions(
        &self,
        query: TransactionQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send> {
        if query.event_matchers.is_empty() {
            return matching_transactions(self.clone(), query);
        }
        event_matching_transactions(self.clone(), query)
    }
}

fn matching_transactions<B>(
    con: B,
    query: TransactionQuery,
) -> Box<dyn Stream<Item = TransactionAndReceipt, Error = ()> + Send>
where
    B: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + BlockByHash<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + ReceiptByHash<
            Receipt = Option<ethereum_support::TransactionReceipt>,
            TransactionHash = ethereum_support::H256,
        > + Clone,
{
    let poll_interval = 500;

    log::info!(target: "ethereum::blocksource", "polling for new blocks every {} ms", poll_interval);
    let mut con_cloned = con.clone();
    let stream = Interval::new_interval(Duration::from_millis(poll_interval))
            .map_err(|e| {
                log::warn!(target: "ethereum::blocksource", "error encountered during polling: {:?}", e);
            })
            .and_then(move |_| {
                con_cloned
                    .latest_block()
                    .or_else(|e| {
                        log::debug!(target: "ethereum::blocksource", "error encountered during polling: {:?}", e);
                        Ok(None)
                    })
            })
            .filter_map(|maybe_block| maybe_block)
            .inspect(|block| {
                if let Block { hash: Some(hash), number: Some(number), .. } = block {
                    log::trace!(target: "ethereum::blocksource", "latest block is {:?} at height {}", hash, number);
                }
            });

    let stream = stream
        .map(move |block| {
            let query = query.clone();
            let con_cloned = con.clone();
            tokio::prelude::stream::iter_ok(block.transactions)
                .filter(move |tx| query.matches(&tx))
                .and_then(move |tx| {
                    con_cloned
                        .receipt_by_hash(tx.hash)
                        .map(move |receipt| {
                            Some(TransactionAndReceipt {
                                transaction: tx,
                                receipt: receipt
                                    .expect("all valid transactions should have a receipt"),
                            })
                        })
                        .or_else(|err| {
                            log::error!("failed to get receipt for transaction: {:?}", err);
                            Ok(None)
                        })
                })
                .filter_map(|maybe| maybe)
        })
        .flatten();

    Box::new(stream)
}

fn event_matching_transactions<B>(
    con: B,
    query: TransactionQuery,
) -> Box<dyn Stream<Item = TransactionAndReceipt, Error = ()> + Send>
where
    B: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + BlockByHash<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + ReceiptByHash<
            Receipt = Option<ethereum_support::TransactionReceipt>,
            TransactionHash = ethereum_support::H256,
        > + Clone,
{
    let poll_interval = 500;

    log::info!(target: "ethereum::blocksource", "polling for new blocks every {} ms", poll_interval);

    let mut cloned_con = con.clone();
    let stream = Interval::new_interval(Duration::from_millis(poll_interval))
        .map_err(|e| {
            log::warn!(target: "ethereum::blocksource", "error encountered during polling: {:?}", e);
        })
        .and_then({
            move |_| {
                cloned_con
                    .latest_block()
                    .or_else(|e| {
                        log::debug!(target: "ethereum::blocksource", "error encountered during receipt polling: {:?}", e);
                        Ok(None)
                    })
            }
        })
        .filter_map(|maybe_block| maybe_block)
        .inspect(|block| {
            if let Block { hash: Some(hash), number: Some(number), .. } = block {
                log::trace!(target: "ethereum::blocksource", "latest block is {:?} at height {}", hash, number);
            }
        });

    let stream = stream
        .filter({
            let query = query.clone();
            move |block| query.event_matches_block(block)
        })
        .map({
            move |block| {
                let result_futures = block.transactions.into_iter().map({
                    let query = query.clone();
                    let cloned_con = con.clone();

                    move |transaction| {
                        let transaction_id = transaction.hash;
                        cloned_con.receipt_by_hash(transaction_id).then({
                            let query = query.clone();

                            move |result| match result {
                                Ok(Some(ref receipt))
                                    if query.event_matches_transaction_receipt(receipt.clone()) =>
                                {
                                    Ok(Some(TransactionAndReceipt {
                                        transaction,
                                        receipt: receipt.clone(),
                                    }))
                                }
                                Err(e) => {
                                    log::error!(
                                        "Could not retrieve transaction receipt for {}: {:?}",
                                        transaction_id,
                                        e
                                    );
                                    Ok(None)
                                }
                                _ => Ok(None),
                            }
                        })
                    }
                });

                tokio::prelude::stream::futures_unordered(result_futures)
            }
        })
        .flatten()
        .filter_map(|maybe_tx| maybe_tx);

    Box::new(stream)
}
