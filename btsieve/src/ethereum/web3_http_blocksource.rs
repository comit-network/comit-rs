use crate::{
    blocksource::{BlockSource, TransactionReceiptBlockSource},
    ethereum::{EventQuery, TransactionQuery},
    matching_transactions::MatchingTransactions,
    web3::{
        self,
        futures::{Future, Stream},
        transports::Http,
        types::{Block, BlockId},
        Web3,
    },
};
use ethereum_support::{Network, TransactionAndReceipt, TransactionId};
use std::{sync::Arc, time::Duration};
use tokio::timer::Interval;
use web3::types::BlockNumber;

#[derive(Debug)]
pub enum Error {
    Web3(web3::Error),
}

#[derive(Clone)]
pub struct Web3HttpBlockSource {
    web3: Arc<Web3<Http>>,
    network: Network,
}

impl Web3HttpBlockSource {
    pub fn new(web3: Arc<Web3<Http>>, network: Network) -> Self {
        Self { web3, network }
    }
}

impl BlockSource for Web3HttpBlockSource {
    type Error = web3::Error;
    type Block = Option<ethereum_support::Block<ethereum_support::Transaction>>;
    type BlockHash = ethereum_support::H256;
    type TransactionHash = ethereum_support::H256;
    type Transaction = Option<ethereum_support::Transaction>;
    type Network = ethereum_support::Network;

    fn network(&self) -> Self::Network {
        self.network
    }

    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(
            web.eth()
                .block_with_txs(BlockId::Number(BlockNumber::Latest)),
        )
    }

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().block_with_txs(BlockId::Hash(block_hash)))
    }

    fn transaction_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Transaction, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().transaction(TransactionId::Hash(transaction_hash)))
    }
}

impl TransactionReceiptBlockSource for Web3HttpBlockSource {
    type TransactionReceipt = Option<ethereum_support::TransactionReceipt>;

    fn transaction_receipt(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::TransactionReceipt, Error = Self::Error> + Send + 'static>
    {
        let web = self.web3.clone();
        Box::new(web.eth().transaction_receipt(transaction_hash))
    }
}

impl<B> MatchingTransactions<TransactionQuery> for Arc<B>
where
    B: BlockSource<
            Block = Option<ethereum_support::Block<ethereum_support::Transaction>>,
            Network = ethereum_support::Network,
        > + Send
        + Sync
        + 'static,
{
    type Transaction = ethereum_support::Transaction;

    fn matching_transactions(
        &self,
        query: TransactionQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send> {
        let poll_interval = match self.network() {
            Network::Mainnet => 5000,
            Network::Ropsten => 5000,
            Network::Regtest => 500,
            Network::Unknown => 1000,
        };

        log::info!(target: "ethereum::blocksource", "polling for new blocks on {} every {} miliseconds", self.network(), poll_interval);

        let cloned_self = self.clone();

        let stream = Interval::new_interval(Duration::from_millis(poll_interval))
            .map_err(|e| {
                log::warn!(target: "ethereum::blocksource", "error encountered during polling: {:?}", e);
            })
            .and_then(move |_| {
                cloned_self
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
                block
                    .transactions
                    .into_iter()
                    .filter(|tx| query.matches(&tx))
                    .collect::<Vec<Self::Transaction>>()
            })
            .map(futures::stream::iter_ok)
            .flatten();

        Box::new(stream)
    }
}

impl<B> MatchingTransactions<EventQuery> for Arc<B>
where
    B: TransactionReceiptBlockSource<
            Block = Option<ethereum_support::Block<ethereum_support::Transaction>>,
            Network = ethereum_support::Network,
            TransactionHash = ethereum_support::H256,
            TransactionReceipt = Option<ethereum_support::TransactionReceipt>,
        > + Send
        + Sync
        + 'static,
{
    type Transaction = TransactionAndReceipt;

    fn matching_transactions(
        &self,
        query: EventQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send + 'static> {
        let poll_interval = match self.network() {
            Network::Mainnet => 5,
            Network::Ropsten => 5,
            Network::Regtest => 1,
            Network::Unknown => 1,
        };

        log::info!(target: "ethereum::blocksource", "polling for new blocks on {} every {} seconds", self.network(), poll_interval);

        let stream = Interval::new_interval(Duration::from_secs(poll_interval))
            .map_err(|e| {
                log::warn!(target: "ethereum::blocksource", "error encountered during polling: {:?}", e);
            })
            .and_then({
                let cloned_self = self.clone();

                move |_| {
                    cloned_self
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

                move |block| query.matches_block(block)
            })
            .map({
                let cloned_self = self.clone();
                move |block| {
                    let result_futures = block.transactions.into_iter().map({
                        let query = query.clone();
                        let cloned_self = cloned_self.clone();

                        move |transaction| {
                            let transaction_id = transaction.hash;
                            cloned_self.transaction_receipt(transaction_id).then({
                                let query = query.clone();

                                move |result| match result {
                                    Ok(Some(ref receipt))
                                        if query.matches_transaction_receipt(receipt.clone()) =>
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

                    futures::stream::futures_unordered(result_futures)
                }
            })
            .flatten()
            .filter_map(|maybe_tx| maybe_tx);

        Box::new(stream)
    }
}
