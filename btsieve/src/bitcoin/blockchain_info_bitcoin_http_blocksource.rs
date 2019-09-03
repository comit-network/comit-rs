use crate::blocksource::{self, BlockSource};
use bitcoin_support::{deserialize, MinedBlock, Network};
use futures::{Future, Stream};
use reqwest::r#async::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::timer::Interval;

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: String,
    height: u32,
}

#[derive(Debug)]
pub enum Error {
    UnsupportedNetwork(String),
    Reqwest(reqwest::Error),
    Hex(hex::FromHexError),
    BlockDeserialization(String),
}

#[derive(Clone)]
pub struct BlockchainInfoHttpBlockSource {
    client: Client,
}

impl BlockchainInfoHttpBlockSource {
    pub fn new(network: Network) -> Result<Self, Error> {
        // Currently configured for Mainnet only because blockchain.info does not
        // support hex-encoded block retrieval for testnet.

        if network != Network::Mainnet {
            log::error!(
                "Network {} not supported for bitcoin http blocksource",
                network
            );
            return Err(Error::UnsupportedNetwork(format!(
                "Network {} currently not supported for bitcoin http plocksource",
                network
            )));
        }

        Ok(Self {
            client: Client::new(),
        })
    }

    fn latest_block(&self) -> impl Future<Item = MinedBlock, Error = Error> + Send + 'static {
        let cloned_self = self.clone();

        self.latest_block_without_tx()
            .and_then(move |latest_block| {
                cloned_self.raw_hex_block(latest_block.hash, latest_block.height)
            })
    }

    fn latest_block_without_tx(
        &self,
    ) -> impl Future<Item = BlockchainInfoLatestBlock, Error = Error> + Send + 'static {
        let latest_block_url = "https://blockchain.info/latestblock";

        self.client
            .get(latest_block_url)
            .send()
            .map_err(Error::Reqwest)
            .and_then(move |mut response| {
                response
                    .json::<BlockchainInfoLatestBlock>()
                    .map_err(Error::Reqwest)
            })
    }

    fn raw_hex_block(
        &self,
        block_hash: String,
        block_height: u32,
    ) -> impl Future<Item = MinedBlock, Error = Error> + Send + 'static {
        let raw_block_by_hash_url =
            format!("https://blockchain.info/rawblock/{}?format=hex", block_hash);

        self.client
            .get(raw_block_by_hash_url.as_str())
            .send()
            .map_err(Error::Reqwest)
            .and_then(|mut response| response.text().map_err(Error::Reqwest))
            .and_then(|response_text| hex::decode(response_text).map_err(Error::Hex))
            .and_then(|bytes| {
                deserialize(bytes.as_ref()).map_err(|e| {
                    log::error!("Got new block but failed to deserialize it because {:?}", e);
                    Error::BlockDeserialization(format!(
                        "Failed to deserialize the response from blockchain.info into a block: {}",
                        e
                    ))
                })
            })
            .map(move |block| {
                log::trace!("Got {:?}", block);
                MinedBlock::new(block, block_height)
            })
    }
}

impl BlockSource for BlockchainInfoHttpBlockSource {
    type Block = MinedBlock;
    type Error = Error;

    fn blocks(
        &self,
    ) -> Box<dyn Stream<Item = Self::Block, Error = blocksource::Error<Error>> + Send> {
        // https://www.blockchain.com/api/q (= https://www.blockchain.info/api/q) states:
        //  "Please limit your queries to a maximum of 1 every 10 seconds." (29/08/2019)
        //
        // The Bitcoin blockchain has a mining interval of about 10 minutes.
        // The poll interval is configured to once every 5 minutes.
        let poll_interval = 300;

        log::info!(target: "bitcoin::blocksource", "polling for new blocks from blockchain.info on {} every {} seconds", Network::Mainnet, poll_interval);

        let cloned_self = self.clone();

        let stream = Interval::new_interval(Duration::from_secs(poll_interval))
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
                            },
                            Error::Hex(e) => {
                                log::warn!(target: "bitcoin::blocksource", "hex-decode error encountered during polling: {:?}", e);
                                Ok(None)
                            },
                            Error::BlockDeserialization(e) => {
                                log::warn!(target: "bitcoin::blocksource", "block-deserialization error encountered during polling: {:?}", e);
                                Ok(None)
                            },
                            _ => Err(error)
                        }
                    })
                    .map_err(blocksource::Error::Source)
            }).filter_map(|maybe_block| maybe_block);

        Box::new(stream)
    }
}
