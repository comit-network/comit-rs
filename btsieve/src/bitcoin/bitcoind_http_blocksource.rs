use crate::blocksource::{self, BlockSource};
use bitcoin_support::{deserialize, Block, Network, Transaction};
use futures::{Future, Stream};
use reqwest::r#async::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::timer::Interval;

#[derive(Deserialize)]
struct ChainInfo {
    bestblockhash: String,
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Hex(hex::FromHexError),
    BlockDeserialization(bitcoin_support::consensus::encode::Error),
    TransactionDeserialization(String),
}

#[derive(Clone)]
pub struct BitcoindHttpBlockSource {
    network: Network,
    base_url: String,
    client: Client,
}

impl BitcoindHttpBlockSource {
    pub fn new(url: String, network: Network) -> Self {
        Self {
            network,
            base_url: url,
            client: Client::new(),
        }
    }

    fn latest_block(&self) -> impl Future<Item = Block, Error = Error> + Send + 'static {
        let cloned_self = self.clone();

        self.latest_block_hash()
            .and_then(move |latest_block_hash| cloned_self.block_by_hash(latest_block_hash))
    }

    fn latest_block_hash(&self) -> impl Future<Item = String, Error = Error> + Send + 'static {
        let bitcoind_blockchain_info_url = format!("{}/rest/chaininfo.json", self.base_url);

        self.client
            .get(bitcoind_blockchain_info_url.as_str())
            .send()
            .map_err(|e| {
                log::error!("Error when sending request to bitcoind");
                Error::Reqwest(e)
            })
            .and_then(move |mut response| {
                response.json::<ChainInfo>().map_err(|e| {
                    log::error!("Error when deserialising the response from bitcoind");
                    Error::Reqwest(e)
                })
            })
            .map(move |blockchain_info| blockchain_info.bestblockhash)
    }

    fn block_by_hash(
        &self,
        block_hash: String,
    ) -> impl Future<Item = Block, Error = Error> + Send + 'static {
        let raw_block_by_hash_url = format!("{}/rest/block/{}.hex", self.base_url, block_hash);

        self.client
            .get(raw_block_by_hash_url.as_str())
            .send()
            .map_err(Error::Reqwest)
            .and_then(|mut response| response.text().map_err(Error::Reqwest))
            .and_then(move |mut response_text| {
                response_text = response_text.as_str().trim().to_string();
                hex::decode(response_text).map_err(Error::Hex)
            })
            .and_then(|bytes| deserialize(bytes.as_ref()).map_err(Error::BlockDeserialization))
            .map(move |block| {
                log::trace!("Got {:?}", block);
                block
            })
    }

    pub fn transaction_by_hash(
        &self,
        transaction_hash: String,
    ) -> impl Future<Item = Transaction, Error = Error> + Send + 'static {
        let raw_transaction_by_hash_url =
            format!("{}/rest/tx/{}.hex", self.base_url, transaction_hash);

        self.client
            .get(raw_transaction_by_hash_url.as_str())
            .send()
            .map_err(Error::Reqwest)
            .and_then(|mut response| response.text().map_err(Error::Reqwest))
            .and_then(move |mut response_text| {
                response_text = response_text.as_str().trim().to_string();
                hex::decode(response_text).map_err(Error::Hex)
            })
            .and_then(|bytes| {
                deserialize(bytes.as_ref()).map_err(|e| {
                    log::error!(
                        "Got new transaction but failed to deserialize it because {:?}",
                        e
                    );
                    Error::TransactionDeserialization(format!(
                        "Failed to deserialize the response from bitcoind into a transaction: {}",
                        e
                    ))
                })
            })
            .inspect(move |transaction| {
                log::debug!("Fetched transaction {:?}", transaction);
            })
    }
}

impl BlockSource for BitcoindHttpBlockSource {
    type Block = Block;
    type Error = Error;

    fn blocks(
        &self,
    ) -> Box<dyn Stream<Item = Self::Block, Error = blocksource::Error<Error>> + Send> {
        // The Bitcoin blockchain has a mining interval of about 10 minutes.
        // The poll interval is configured to once every 2 minutes for mainnet and
        // testnet so we don't have to wait to long to see a new block.
        let poll_interval = match self.network {
            Network::Mainnet => 120_000,
            Network::Testnet => 120_000,
            Network::Regtest => 300,
        };

        log::info!(target: "bitcoin::blocksource", "polling for new blocks from bitcoind on {} every {} seconds", self.network, poll_interval);

        let cloned_self = self.clone();

        let stream = Interval::new_interval(Duration::from_millis(poll_interval))
            .map_err(blocksource::Error::Timer)
            .and_then(move |_| {
                cloned_self
                    .latest_block()
                    .map(Some)
                    .or_else(|error| {
                        match error {
                            Error::Reqwest(e) => {
                                log::warn!(target: "bitcoin::blocksource", "reqwest error encountered during polling: {:?}", e);
                                Ok(None)
                            }
                            Error::Hex(e) => {
                                log::warn!(target: "bitcoin::blocksource", "hex-decode error encountered during polling: {:?}", e);
                                Ok(None)
                            }
                            Error::BlockDeserialization(e) => {
                                log::warn!(target: "bitcoin::blocksource", "block-deserialization error encountered during polling: {:?}", e);
                                Ok(None)
                            }
                            _ => Err(error)
                        }
                    })
                    .map_err(blocksource::Error::Source)
            })
            .filter_map(|maybe_block| maybe_block);

        Box::new(stream)
    }
}
