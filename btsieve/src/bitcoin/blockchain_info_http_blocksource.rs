use crate::blocksource::{self, BlockSource};
use bitcoin_support::{deserialize, MinedBlock, Network};
use futures::{Future, Stream};
use reqwest::r#async::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::timer::Interval;
use url::{ParseError, Url};

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: String,
    height: u32,
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Hex(hex::FromHexError),
    BlockDeserialization(String),
    ParseUrl(ParseError),
}

#[derive(Clone)]
pub struct BlockchainInfoHttpBlockSource {
    bitcoin_block_http_api: url::Url,
    network: Network,
    client: Client,
}

// and return None in some scenarios
impl BlockchainInfoHttpBlockSource {
    pub fn new(network: Network) -> Self {
        // blockchain.info and blockchain.com are not really the same
        // The API is only available under blockchain.info
        // Mainnet: blockchain.info
        // Testnet: testnet.blockchain.info

        let bitcoin_block_http_api: Url;

        match network {
            Network::Mainnet => {
                bitcoin_block_http_api = Url::parse("https://blockchain.info")
                    .map_err(Error::ParseUrl)
                    .unwrap();
            }
            Network::Testnet => {
                bitcoin_block_http_api = Url::parse("https://testnet.blockchain.info")
                    .map_err(Error::ParseUrl)
                    .unwrap();
            }
            _ => {
                log::error!(
                    "Network {} not supported for bitcoin http blocksource",
                    network
                );
                panic!(
                    "Network {} not supported for bitcoin http blocksource",
                    network
                )
            }
        };

        Self {
            bitcoin_block_http_api,
            network,
            client: Client::new(),
        }
    }

    pub fn latest_block(&self) -> impl Future<Item = MinedBlock, Error = Error> + Send + 'static {
        let cloned_self = self.clone();

        self.latest_block_without_tx()
            .and_then(move |latest_block| {
                cloned_self.raw_hex_block(latest_block.hash, latest_block.height)
            })
    }

    fn latest_block_without_tx(
        &self,
    ) -> impl Future<Item = BlockchainInfoLatestBlock, Error = Error> + Send + 'static {
        // https://blockchain.info/q/latesthash only works for mainnet, there is no testnet endpoint
        // we fall-back to [testnet.]blockchain.info/latestblock to retrieve the latest
        // block hash

        let latest_block_url = self
            .bitcoin_block_http_api
            .join("latestblock")
            .map_err(Error::ParseUrl)
            .unwrap();

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
        // TODO: Put this in the constructor, let the constructor return a result, then
        // cascade these using ?
        let block_url = self
            .bitcoin_block_http_api
            .join("rawblock/")
            .map_err(Error::ParseUrl)
            .unwrap();
        let block_by_hash_url = block_url
            .join(block_hash.as_str())
            .map_err(Error::ParseUrl)
            .unwrap();
        let raw_block_by_hash_url = block_by_hash_url
            .join("?format=hex")
            .map_err(Error::ParseUrl)
            .unwrap();

        self.client
            .get(raw_block_by_hash_url)
            .send()
            .map_err(Error::Reqwest)
            .and_then(|mut response| response.text().map_err(Error::Reqwest))
            .and_then(|response_text| hex::decode(response_text).map_err(Error::Hex))
            .and_then(move |bytes| {
                deserialize(bytes.as_ref())
                    .map(|block| {
                        log::trace!("Got {:?}", block);
                        MinedBlock::new(block, block_height)
                    })
                    .map_err(|e| {
                        log::error!("Got new block but failed to deserialize it because {:?}", e);
                        Error::BlockDeserialization(format!(
                            "Failed to deserialize the resonse from blockchain.info into a block: {}", e
                        ))
                    })
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
        let poll_interval = match self.network {
            Network::Mainnet => 300,
            Network::Testnet => 300,
            Network::Regtest => 1,
        };

        log::info!(target: "bitcoin::blocksource", "polling for new blocks from {} on {} every {} seconds", self.bitcoin_block_http_api, self.network, poll_interval);

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
