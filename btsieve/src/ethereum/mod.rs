mod queries;
mod web3_connector;

#[cfg(test)]
mod quickcheck_impls;

pub use self::{
    queries::{Event, Topic, TransactionQuery},
    web3_connector::Web3Connector,
};
use crate::{BlockByHash, LatestBlock, MatchingTransactions, ReceiptByHash};
use ethereum_support::TransactionAndReceipt;
use futures::{compat::Future01CompatExt, TryFutureExt};
use std::ops::Add;
use tokio::{
    prelude::{stream, Stream},
    timer::Delay,
};

impl<C> MatchingTransactions<TransactionQuery> for C
where
    C: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
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
        let matching_transaction = Box::pin(matching_transaction(self.clone(), query)).compat();
        Box::new(stream::futures_unordered(vec![matching_transaction]))
    }
}

async fn matching_transaction<C>(
    mut blockchain_connector: C,
    query: TransactionQuery,
) -> Result<TransactionAndReceipt, ()>
where
    C: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + BlockByHash<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + ReceiptByHash<
            Receipt = Option<ethereum_support::TransactionReceipt>,
            TransactionHash = ethereum_support::H256,
        > + Clone,
{
    loop {
        // Delay so that we don't overload the machine given the assumption
        // that latest_block and block_by_hash resolve quickly
        Delay::new(std::time::Instant::now().add(std::time::Duration::from_secs(1)))
            .compat()
            .await
            .unwrap_or_else(|e| log::warn!("Failed to wait for delay: {:?}", e));

        let latest_block = match blockchain_connector.latest_block().compat().await {
            Ok(Some(block)) => block,
            Ok(None) => {
                log::warn!("Could not get latest block");
                continue;
            }
            Err(e) => {
                log::warn!("Could not get latest block: {:?}", e);
                continue;
            }
        };

        if query.can_skip_block(&latest_block) {
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

            if query.matches(transaction) || query.event_matches_transaction_receipt(&receipt) {
                return Ok(TransactionAndReceipt {
                    transaction: transaction.clone(),
                    receipt,
                });
            };
        }
    }
}
