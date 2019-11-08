mod transaction_pattern;
mod web3_connector;

pub use self::{
    transaction_pattern::{Event, Topic, TransactionPattern},
    web3_connector::Web3Connector,
};
use crate::{
    btsieve::{BlockByHash, LatestBlock, MatchingTransactions, ReceiptByHash},
    ethereum::{TransactionAndReceipt, H256},
};
use futures_core::{compat::Future01CompatExt, TryFutureExt};
use std::{collections::HashSet, fmt::Debug, ops::Add};
use tokio::{
    prelude::{stream, Stream},
    timer::Delay,
};

impl<C, E> MatchingTransactions<TransactionPattern> for C
where
    C: LatestBlock<Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>, Error = E>
        + BlockByHash<
            Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>,
            BlockHash = H256,
            Error = E,
        > + ReceiptByHash<
            Receipt = Option<crate::ethereum::TransactionReceipt>,
            TransactionHash = crate::ethereum::H256,
            Error = E,
        > + Clone,
    E: Debug + Send + 'static,
{
    type Transaction = TransactionAndReceipt;

    fn matching_transactions(
        &self,
        pattern: TransactionPattern,
        _timestamp: Option<u32>,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send> {
        let matching_transaction = Box::pin(matching_transaction(self.clone(), pattern)).compat();
        Box::new(stream::futures_unordered(vec![matching_transaction]))
    }
}

async fn matching_transaction<C, E>(
    mut blockchain_connector: C,
    pattern: TransactionPattern,
) -> Result<TransactionAndReceipt, ()>
where
    C: LatestBlock<Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>, Error = E>
        + BlockByHash<
            Block = Option<crate::ethereum::Block<crate::ethereum::Transaction>>,
            BlockHash = H256,
            Error = E,
        > + ReceiptByHash<
            Receipt = Option<crate::ethereum::TransactionReceipt>,
            TransactionHash = crate::ethereum::H256,
            Error = E,
        > + Clone,
    E: Debug + Send + 'static,
{
    let mut prev_blockhashes: HashSet<H256> = HashSet::new();
    let mut missing_block_futures: Vec<_> = Vec::new();

    loop {
        // Delay so that we don't overload the CPU in the event that
        // latest_block() and block_by_hash() resolve quickly.
        Delay::new(std::time::Instant::now().add(std::time::Duration::from_secs(1)))
            .compat()
            .await
            .unwrap_or_else(|e| log::warn!("Failed to wait for delay: {:?}", e));

        let mut new_missing_block_futures = Vec::new();
        for (block_future, blockhash) in missing_block_futures.into_iter() {
            match block_future.await {
                Ok(Some(block)) => {
                    let block: crate::ethereum::Block<crate::ethereum::Transaction> = block;
                    for transaction in block.transactions.iter() {
                        let result = blockchain_connector
                            .receipt_by_hash(transaction.hash)
                            .compat()
                            .await;

                        let receipt = match result {
                            Ok(Some(receipt)) => receipt,
                            Ok(None) => {
                                log::warn!("Could not get transaction receipt");
                                continue;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Could not retrieve transaction receipt for {}: {:?}",
                                    transaction.hash,
                                    e
                                );
                                continue;
                            }
                        };

                        if pattern.matches(transaction, &receipt) {
                            return Ok(TransactionAndReceipt {
                                transaction: transaction.clone(),
                                receipt,
                            });
                        };
                    }

                    let prev_blockhash = block.parent_hash;
                    let unknown_parent = prev_blockhashes.insert(prev_blockhash);

                    if unknown_parent {
                        let future = blockchain_connector.block_by_hash(prev_blockhash).compat();
                        new_missing_block_futures.push((future, prev_blockhash));
                    }
                }
                Ok(None) => {
                    log::warn!("Could not get block with hash {}", blockhash);

                    let future = blockchain_connector.block_by_hash(blockhash).compat();
                    new_missing_block_futures.push((future, blockhash));
                }
                Err(e) => {
                    log::warn!("Could not get block with hash {}: {:?}", blockhash, e);

                    let future = blockchain_connector.block_by_hash(blockhash).compat();
                    new_missing_block_futures.push((future, blockhash));
                }
            };
        }
        missing_block_futures = new_missing_block_futures;

        let latest_block = match blockchain_connector.latest_block().compat().await {
            Ok(Some(block)) if block.hash.is_some() => block,
            Ok(Some(_)) => {
                log::warn!("Ignoring block without blockhash");
                continue;
            }
            Ok(None) => {
                log::warn!("Could not get latest block");
                continue;
            }
            Err(e) => {
                log::warn!("Could not get latest block: {:?}", e);
                continue;
            }
        };

        // If we can't insert then we have seen this block
        if !prev_blockhashes.insert(latest_block.hash.expect("cannot fail")) {
            continue;
        }

        if prev_blockhashes.len() > 1 && !prev_blockhashes.contains(&latest_block.parent_hash) {
            let prev_blockhash = latest_block.parent_hash;
            let future = blockchain_connector.block_by_hash(prev_blockhash).compat();

            missing_block_futures.push((future, prev_blockhash));
        }

        if pattern.can_skip_block(&latest_block) {
            continue;
        }

        for transaction in latest_block.transactions.iter() {
            let result = blockchain_connector
                .receipt_by_hash(transaction.hash)
                .compat()
                .await;

            let receipt = match result {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    log::warn!("Could not get transaction receipt");
                    continue;
                }
                Err(e) => {
                    log::warn!(
                        "Could not retrieve transaction receipt for {}: {:?}",
                        transaction.hash,
                        e
                    );
                    continue;
                }
            };

            if pattern.matches(transaction, &receipt) {
                return Ok(TransactionAndReceipt {
                    transaction: transaction.clone(),
                    receipt,
                });
            };
        }
    }
}
