use crate::{
    ethereum::{EventQuery, TransactionQuery},
    BlockByHash, LatestBlock, MatchingTransactions, ReceiptByHash,
};
use ethereum_support::{
    web3::{
        self,
        futures::{Future, Stream},
        transports::{EventLoopHandle, Http},
        types::{Block, BlockId},
        Web3,
    },
    BlockNumber, Network, TransactionAndReceipt,
};
use reqwest::Url;
use std::{sync::Arc, time::Duration};
use tokio::timer::Interval;

pub struct Web3Connector {
    _event_loop_handle: EventLoopHandle,
    web3: Arc<Web3<Http>>,
}

impl Web3Connector {
    pub fn new(node_url: Url, _network: Network) -> Result<Self, web3::Error> {
        let (event_loop_handle, http_transport) = Http::new(node_url.as_str())?;
        Ok(Self {
            _event_loop_handle: event_loop_handle,
            web3: Arc::new(Web3::new(http_transport)),
        })
    }
}

impl LatestBlock for Web3Connector {
    type Error = web3::Error;
    type Block = Option<ethereum_support::Block<ethereum_support::Transaction>>;
    type BlockHash = ethereum_support::H256;

    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(
            web.eth()
                .block_with_txs(BlockId::Number(BlockNumber::Latest)),
        )
    }
}

impl BlockByHash for Web3Connector {
    type Error = web3::Error;
    type Block = Option<ethereum_support::Block<ethereum_support::Transaction>>;
    type BlockHash = ethereum_support::H256;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().block_with_txs(BlockId::Hash(block_hash)))
    }
}

impl ReceiptByHash for Web3Connector {
    type Receipt = Option<ethereum_support::TransactionReceipt>;
    type TransactionHash = ethereum_support::H256;
    type Error = web3::Error;

    fn receipt_by_hash(
        &self,
        transaction_hash: Self::TransactionHash,
    ) -> Box<dyn Future<Item = Self::Receipt, Error = Self::Error> + Send + 'static> {
        let web = self.web3.clone();
        Box::new(web.eth().transaction_receipt(transaction_hash))
    }
}

impl<B> MatchingTransactions<TransactionQuery> for Arc<B>
where
    B: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + BlockByHash<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>,
{
    type Transaction = ethereum_support::Transaction;

    fn matching_transactions(
        &self,
        query: TransactionQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send> {
        let poll_interval = 500;

        log::info!(target: "ethereum::blocksource", "polling for new blocks every {} ms", poll_interval);

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
    B: LatestBlock<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + BlockByHash<Block = Option<ethereum_support::Block<ethereum_support::Transaction>>>
        + ReceiptByHash<
            Receipt = Option<ethereum_support::TransactionReceipt>,
            TransactionHash = ethereum_support::H256,
        >,
{
    type Transaction = TransactionAndReceipt;

    fn matching_transactions(
        &self,
        query: EventQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send + 'static> {
        let poll_interval = 500;

        log::info!(target: "ethereum::blocksource", "polling for new blocks every {} ms", poll_interval);

        let stream = Interval::new_interval(Duration::from_millis(poll_interval))
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
                            cloned_self.receipt_by_hash(transaction_id).then({
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
