use crate::{
    bitcoin::{self, bitcoin_http_request_for_hex_encoded_object},
    BlockByHash, LatestBlock,
};
use bitcoin_support::Network;
use futures::Future;
use reqwest::{r#async::Client, Url};
use serde::Deserialize;

#[derive(Deserialize)]
struct ChainInfo {
    bestblockhash: bitcoin_support::BlockId,
}

#[derive(Clone)]
pub struct BitcoindConnector {
    chaininfo_url: Url,
    raw_block_by_hash_url: Url,
    client: Client,
}

impl BitcoindConnector {
    pub fn new(base_url: Url, _network: Network) -> Result<Self, reqwest::UrlError> {
        Ok(Self {
            chaininfo_url: base_url.join("rest/chaininfo.json")?,
            raw_block_by_hash_url: base_url.join("rest/block/")?,
            client: Client::new(),
        })
    }

    fn raw_block_by_hash_url(&self, block_hash: &bitcoin_support::BlockId) -> Url {
        self.raw_block_by_hash_url
            .join(&format!("{}.hex", block_hash))
            .expect("building url should work")
    }
}

impl LatestBlock for BitcoindConnector {
    type Error = bitcoin::Error;
    type Block = bitcoin_support::Block;
    type BlockHash = bitcoin_support::BlockId;

    fn latest_block(
        &self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let latest_block_hash = self
            .client
            .get(self.chaininfo_url.clone())
            .send()
            .map_err(|e| {
                log::error!("Error when sending request to bitcoind");
                Self::Error::Reqwest(e)
            })
            .and_then(move |mut response| {
                response.json::<ChainInfo>().map_err(|e| {
                    log::error!("Error when deserialising the response from bitcoind");
                    Self::Error::Reqwest(e)
                })
            })
            .map(move |blockchain_info| blockchain_info.bestblockhash);

        let cloned_self = self.clone();

        Box::new(
            latest_block_hash
                .and_then(move |latest_block_hash| cloned_self.block_by_hash(latest_block_hash)),
        )
    }
}

impl BlockByHash for BitcoindConnector {
    type Error = bitcoin::Error;
    type Block = bitcoin_support::Block;
    type BlockHash = bitcoin_support::BlockId;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let url = self.raw_block_by_hash_url(&block_hash);

        let block =
            bitcoin_http_request_for_hex_encoded_object::<Self::Block>(url, self.client.clone());

        Box::new(block.inspect(|block| {
            log::trace!("Fetched block from bitcoind: {:?}", block);
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::quickcheck::Quickcheck;

    fn base_urls() -> Vec<Url> {
        vec![
            "http://localhost:8080".parse().unwrap(),
            "http://localhost:8080/".parse().unwrap(),
        ]
    }

    #[test]
    fn constructor_does_not_fail_for_base_urls() {
        for base_url in base_urls() {
            let result = BitcoindConnector::new(base_url, Network::Regtest);

            assert!(result.is_ok());
        }
    }

    // This quickcheck test asserts that we can feed arbitrary input to these
    // functions and they never panic, hence it is fine to use them in production
    #[test]
    fn build_sub_url_should_never_fail() {
        fn prop(hash: Quickcheck<bitcoin_support::BlockId>) -> bool {
            for base_url in base_urls() {
                let blocksource = BitcoindConnector::new(base_url, Network::Regtest).unwrap();

                blocksource.raw_block_by_hash_url(&hash);
            }

            true // not panicing is good enough for this test
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<bitcoin_support::BlockId>) -> bool)
    }

    #[test]
    fn given_different_base_urls_correct_sub_urls_are_built() {
        for base_url in base_urls() {
            let blocksource = BitcoindConnector::new(base_url, Network::Regtest).unwrap();

            let chaininfo_url = blocksource.chaininfo_url.clone();
            assert_eq!(
                chaininfo_url,
                Url::parse("http://localhost:8080/rest/chaininfo.json").unwrap()
            );

            let block_id = "2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02"
                .parse()
                .unwrap();
            let raw_block_by_hash_url = blocksource.raw_block_by_hash_url(&block_id);
            assert_eq!(raw_block_by_hash_url, Url::parse("http://localhost:8080/rest/block/2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02.hex").unwrap());
        }
    }
}
