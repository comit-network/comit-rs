use crate::blocksource::{self, BlockSource};
use bitcoin_support::{
    Block, BlockHeader, FromHex, Network, OutPoint, Sha256dHash, Transaction, TxIn,
};
use failure::Fail;
use futures::{Future, Stream};
use http::StatusCode;
use reqwest::r#async::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::timer::Interval;
use url::Url;
use warp::Filter;

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: String,
    time: u32,
    block_index: u32,
    height: u32,
    tx_indexes: Vec<u32>,
}

#[derive(Deserialize)]
struct BlockchainInfoRawBlock {
    hash: String,
    ver: u32,
    prev_block: String,
    next_block: Vec<String>,
    mrkl_root: String,
    time: u32,
    bits: u32,
    fee: u32,
    nonce: u32,
    n_tx: u32,
    size: u32,
    block_index: u32,
    main_chain: bool,
    height: u32,
    received_time: u32,
    relayed_by: String,
    tx: Vec<BlockchainInfoRawBlockTransaction>,
}

#[derive(Deserialize)]
struct BlockchainInfoRawBlockTransaction {
    lock_time: u32,
    ver: u32,
    size: u32,
    inputs: Vec<BlockchainInfoRawBlockTransactionInput>,
    weight: u32,
    time: u32,
    tx_index: u32,
    vin_sz: u32,
    hash: String,
    vout_sz: u32,
    relayed_by: String,
    out: Vec<BlockchainInfoRawBlockTransactionOutput>,
}

#[derive(Deserialize)]
struct BlockchainInfoRawBlockTransactionInput {
    sequence: u32,
    witness: Vec<u8>,
    prev_out: Option<BlockchainInfoRawBlockTransactionOutput>,
    script: Vec<u8>,
}

#[derive(Deserialize)]
struct BlockchainInfoRawBlockTransactionOutput {
    addr_tag: Option<String>,
    spent: bool,
    spending_outpoints: Vec<BlockchainInfoRawBlockTransactionOutputSpendingOutpoint>,
    tx_index: u32,
    #[serde(alias = "type")]
    tx_out_type: u32,
    addr: String,
    value: u32,
    n: u32,
    script: String,
}

#[derive(Deserialize)]
struct BlockchainInfoRawBlockTransactionOutputSpendingOutpoint {
    tx_index: u32,
    n: u32,
}

// TODO: USE ?format=hex INSTEAD
// Validate that it uses consensus encoding
impl From<BlockchainInfoRawBlock> for Block {
    fn from(raw_block: BlockchainInfoRawBlock) -> Self {
        Block {
            header: BlockHeader {
                version: raw_block.ver,
                prev_blockhash: Sha256dHash::from_hex(raw_block.prev_block.as_str()).map_err(|e| {
                    // TODO: Handle err
                }),
                merkle_root: Sha256dHash::from_hex(raw_block.mrkl_root.as_str()).map_err(|e| {
                    // TODO: Handle err
                }),
                time: raw_block.time,
                bits: raw_block.bits,
                nonce: raw_block.nonce,
            },
            txdata: raw_block
                .tx
                .iter()
                .map(|raw_block_tx| {
                    Transaction {
                        version: raw_block_tx.ver,
                        lock_time: raw_block_tx.lock_time,
                        input: raw_block_tx
                            .inputs
                            .iter()
                            .map(|raw_block_tx_in| {
                                TxIn {
                                    previous_output: raw_block_tx_in
                                        .prev_out
                                        .map(|prev_out| {
                                            OutPoint::null() // TODO: map properly, matching problems
                                        })
                                        .unwrap(), // TODO: fix unwrap
                                    script_sig: Default::default(), // TODO: map properly, matching problems
                                    sequence: raw_block_tx_in.sequence,
                                    witness: vec![], // TODO: map properly, matching problems
                                }
                            })
                            .collect::<Vec<_>>(),
                        output: vec![],
                    }
                })
                .collect::<Vec<_>>(),
        }
    }
}

#[derive(Fail, Debug, PartialEq, Clone)]
pub enum Error {
    #[fail(display = "The request failed to send.")]
    FailedRequest(String),
    #[fail(display = "The response was somehow malformed.")]
    MalformedResponse(String),
}

pub struct BlockchainInfoHttpBlockSource {
    bitcoin_block_http_api: url::Url,
    network: Network,
    client: Client,
}

impl BlockchainInfoHttpBlockSource {
    pub fn new(network: Network) -> Self {
        // blockchain.info and blockchain.com are not really the same
        // The API is only available under blockchain.info
        // Mainnet: blockchain.info
        // Testnet: testnet.blockchain.info

        let bitcoin_block_http_api: Url;

        match network {
            Network::Mainnet => {
                bitcoin_block_http_api = Url::parse("https://blockchain.info").unwrap();
            }
            Network::Testnet => {
                bitcoin_block_http_api = Url::parse("https://testnet.blockchain.info").unwrap();
            }
            _ => {
                // TODO: better error handling
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

    pub fn latest_block() -> Box<dyn Future<Item = Block, Error = blocksource::Error<Error>>> {
        let latest_block_hash = Self::latest_block_hash(Self);

        latest_block_hash.and_then(|hash| Self::raw_block_by_hash(Self, hash))
    }

    fn latest_block_hash(
        &self,
    ) -> Box<dyn Future<Item = String, Error = blocksource::Error<Error>> + Send> {
        // https://blockchain.info/q/latesthash only works for mainnet, there is no testnet endpoint
        // we fall-back to [testnet.]blockchain.info/latestblock to retrieve the latest block hash

        let latest_block_url = self.bitcoin_block_http_api.join("latestblock").unwrap();

        let latest_block_hash = self
            .client
            .get(latest_block_url)
            .send()
            .map_err(move |e| {
                Error::FailedRequest(format!(
                    "Failed to retrieve latest block hash from blockchain.info"
                ))
            })
            .and_then(move |response| {
                if response.status() != StatusCode::OK {
                    // TODO: Error handling for case where the resource is unavailable (e.g. URL changed, ...)
                }

                let
            })
            .inspect(|latest_block_hash| {
                // TODO: can be removed, does not need logging here
                log::info!("Latest block hash for bitcoin is {}", latest_block_hash);
            })
            .map(String::new());

        Box::new(latest_block_hash)
    }

    fn raw_block_by_hash(
        &self,
        block_hash: String,
    ) -> Box<dyn Future<Item = BlockchainInfoRawBlock, Error = blocksource::Error<Error>> + Send>
    {
        let raw_block_url = self.bitcoin_block_http_api.join("rawblock").unwrap();
        let raw_block_by_hash_url = raw_block_url.join(block_hash.as_str()).unwrap();

        let raw_block = self
            .client
            .get(raw_block_by_hash_url)
            .send()
            .map_err(move |e| {
                Error::FailedRequest(format!(
                    "Failed to retrieve latest block hash from blockchain.info"
                ))
            })
            .and_then(move |response| {
                if response.status() != StatusCode::OK {
                    // TODO: Error handling for case where the resource is unavailable (e.g. URL changed, ...)
                }

                response.json::<BlockchainInfoRawBlock>()
            })
            .map(Block::from);

        Box::new(raw_block)
    }
}

impl BlockSource for BlockchainInfoHttpBlockSource {
    type Block = Block;
    type Error = Error;

    fn blocks(
        &self,
    ) -> Box<dyn Stream<Item = Self::Block, Error = blocksource::Error<Error>> + Send> {
        // https://www.blockchain.com/api/q (= https://www.blockchain.info/api/q) states:
        //  "Please limit your queries to a maximum of 1 every 10 seconds." (29/08/2019)
        //
        // Since bitcoin blocks have a mining interval of about 10 minutes the poll interval
        // is configured to once every 5 minutes.
        let poll_interval = match self.network {
            Network::Mainnet => 300,
            Network::Testnet => 300,
            Network::Regtest => 1,
        };

        log::info!(target: "bitcoin::blocksource", "polling for new blocks from {} on {} every {} seconds", self.bitcoin_block_http_api, self.network, poll_interval);

        let stream = Interval::new_interval(Duration::from_secs(poll_interval))
            .map_err(Error::Timer)
            .and_then(move |_| Self::latest_block());
        //            .filter_map(|maybe_block| maybe_block) // TODO: might be obsolete
        //            .inspect(|block| {
        //                if let Block { hash: Some(hash), number: Some(number), .. } = block {
        //                    log::trace!(target: "bitcoin::blocksource", "latest block is {:?} at height {}", hash, number);
        //                }
        //            });

        Box::new(stream)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn latest_block() {}

}
