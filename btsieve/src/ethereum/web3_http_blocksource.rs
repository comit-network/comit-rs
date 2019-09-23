use crate::{
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
use ethereum_support::Network;
use std::{sync::Arc, time::Duration};
use tokio::timer::Interval;
use web3::types::BlockNumber;

#[derive(Debug)]
pub enum Error {
    Timer(tokio::timer::Error),
    Web3(web3::Error),
}

pub struct Web3HttpBlockSource {
    web3: Arc<Web3<Http>>,
    network: Network,
}

impl Web3HttpBlockSource {
    pub fn new(web3: Arc<Web3<Http>>) -> impl Future<Item = Self, Error = web3::Error> {
        web3.clone().net().version().map(move |version| Self {
            web3,
            network: Network::from_network_id(version),
        })
    }
}

impl MatchingTransactions<TransactionQuery> for Web3HttpBlockSource {
    type Error = Error;
    type Transaction = ethereum_support::Transaction;

    fn matching_transactions(
        &self,
        query: TransactionQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = Self::Error> + Send> {
        let web = self.web3.clone();

        let poll_interval = match self.network {
            Network::Mainnet => 5000,
            Network::Ropsten => 5000,
            Network::Regtest => 500,
            Network::Unknown => 500,
        };

        log::info!(target: "ethereum::blocksource", "polling for new blocks on {} every {} miliseconds", self.network, poll_interval);

        let stream = Interval::new_interval(Duration::from_millis(poll_interval))
            .map_err(Error::Timer)
            .and_then(move |_| {
                web.eth()
                    .block_with_txs(BlockId::Number(BlockNumber::Latest))
                    .or_else(|error| {
                        match error {
                            web3::Error::Io(e) => {
                                log::debug!(target: "ethereum::blocksource", "IO error encountered during polling: {:?}", e);
                                Ok(None)
                            },
                            web3::Error::Transport(e)  => {
                                log::debug!(target: "ethereum::blocksource", "Transport error encountered during polling: {:?}", e);
                                Ok(None)
                            },
                            _ => Err(error)
                        }
                    })
                    .map_err(Error::Web3)
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

impl MatchingTransactions<EventQuery> for Web3HttpBlockSource {
    type Error = Error;
    type Transaction = ethereum_support::Transaction;

    fn matching_transactions(
        &self,
        query: EventQuery,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = Self::Error> + Send + 'static> {
        let poll_interval = match self.network {
            Network::Mainnet => 5,
            Network::Ropsten => 5,
            Network::Regtest => 1,
            Network::Unknown => 1,
        };

        log::info!(target: "ethereum::blocksource", "polling for new blocks on {} every {} seconds", self.network, poll_interval);

        let stream = Interval::new_interval(Duration::from_secs(poll_interval))
            .map_err(Error::Timer)
            .and_then({
                let web3 = self.web3.clone();

                move |_| {
                    web3.eth()
                        .block_with_txs(BlockId::Number(BlockNumber::Latest))
                        .or_else(|error| {
                            match error {
                                web3::Error::Io(e) => {
                                    log::debug!(target: "ethereum::blocksource", "IO error encountered during polling: {:?}", e);
                                    Ok(None)
                                },
                                web3::Error::Transport(e)  => {
                                    log::debug!(target: "ethereum::blocksource", "Transport error encountered during polling: {:?}", e);
                                    Ok(None)
                                },
                                _ => Err(error)
                            }
                        })
                        .map_err(Error::Web3)
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
                let web3 = self.web3.clone();

                move |block| {
                    let result_futures = block.transactions.into_iter().map({
                        let query = query.clone();
                        let web3 = web3.clone();

                        move |transaction| {
                            let transaction_id = transaction.hash;
                            web3.eth().transaction_receipt(transaction_id).then({
                                let query = query.clone();

                                move |result| match result {
                                    Ok(Some(ref receipt))
                                        if query.matches_transaction_receipt(receipt.clone()) =>
                                    {
                                        Ok(Some(transaction))
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Could not retrieve transaction receipt for {}: {}",
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
