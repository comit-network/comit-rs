use crate::blocksource::{self, BlockSource};
use bitcoin_support::{consensus::Decodable, deserialize, Block, Network, Transaction};
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
    Deserialization(bitcoin_support::consensus::encode::Error),
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
            .and_then(|mut response| response.text())
            .map_err(Error::Reqwest)
            .and_then(decode_response)
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
            .and_then(|mut response| response.text())
            .map_err(Error::Reqwest)
            .and_then(decode_response)
            .map(move |transaction| {
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
                            Error::Deserialization(e) => {
                                log::warn!(target: "bitcoin::blocksource", "deserialization error encountered during polling: {:?}", e);
                                Ok(None)
                            }
                        }
                    })
                    .map_err(blocksource::Error::Source)
            })
            .filter_map(|maybe_block| maybe_block);

        Box::new(stream)
    }
}

fn decode_response<T: Decodable>(response_text: String) -> Result<T, Error> {
    let bytes = hex::decode(response_text.trim()).map_err(Error::Hex)?;

    deserialize(bytes.as_slice()).map_err(Error::Deserialization)
}

#[cfg(test)]
mod tests {

    use super::*;
    use spectral::prelude::*;

    #[test]
    fn can_decode_tx_from_bitcoind_http_interface() {
        // the line break here is on purpose, as it is returned like that from bitcoind
        let transaction = r#"02000000014135047eff77c95bce4955f630bc3e334690d31517176dbc23e9345493c48ecf000000004847304402200da78118d6970bca6f152a6ca81fa8c4dde856680eb6564edb329ce1808207c402203b3b4890dd203cc4c9361bbbeb7ebce70110d4b07f411208b2540b10373755ba01feffffff02644024180100000017a9142464790f3a3fddb132691fac9fd02549cdc09ff48700a3e1110000000017a914c40a2c4fd9dcad5e1694a41ca46d337eb59369d78765000000
"#.to_owned();

        let bytes = decode_response::<Transaction>(transaction);

        assert_that(&bytes).is_ok();
    }

    #[test]
    fn can_decode_block_from_bitcoind_http_interface() {
        // the line break here is on purpose, as it is returned like that from bitcoind
        let transaction = r#"00000020837603de6069115e22e7fbf063c2a6e3bc3b3206f0b7e08d6ab6c168c2e50d4a9b48676dedc93d05f677778c1d83df28fd38d377548340052823616837666fb8be1b795dffff7f200000000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401650101ffffffff0200f2052a0100000023210205980e76eee77386241a3a7a5af65e910fb7be411b98e609f7c0d97c50ab8ebeac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000
"#.to_owned();

        let bytes = decode_response::<Block>(transaction);

        assert_that(&bytes).is_ok();
    }
}
