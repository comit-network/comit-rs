use crate::btsieve::{
    bitcoin::bitcoin_http_request_for_hex_encoded_object, BlockByHash, LatestBlock,
};
use bitcoin::{BlockHash, Network};
use futures::Future;
use futures_core::{compat::Future01CompatExt, FutureExt, TryFutureExt};
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Deserialize)]
struct ChainInfo {
    bestblockhash: BlockHash,
}

#[derive(Clone, Debug)]
pub struct BitcoindConnector {
    chaininfo_url: Url,
    raw_block_by_hash_url: Url,
    client: Client,
}

impl BitcoindConnector {
    pub fn new(base_url: Url, _network: Network) -> anyhow::Result<Self> {
        Ok(Self {
            chaininfo_url: base_url.join("rest/chaininfo.json")?,
            raw_block_by_hash_url: base_url.join("rest/block/")?,
            client: Client::new(),
        })
    }

    fn raw_block_by_hash_url(&self, block_hash: &BlockHash) -> Url {
        self.raw_block_by_hash_url
            .join(&format!("{}.hex", block_hash))
            .expect("building url should work")
    }
}

impl LatestBlock for BitcoindConnector {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let chaininfo_url = self.chaininfo_url.clone();
        let this = self.clone();

        let latest_block = async move {
            let chain_info = this
                .client
                .get(chaininfo_url)
                .send()
                .await?
                .json::<ChainInfo>()
                .await?;

            let block = this
                .block_by_hash(chain_info.bestblockhash)
                .compat()
                .await?;

            Ok(block)
        };

        Box::new(latest_block.boxed().compat())
    }
}

impl BlockByHash for BitcoindConnector {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = anyhow::Error> + Send + 'static> {
        let url = self.raw_block_by_hash_url(&block_hash);

        let client = self.client.clone();
        let block = async move {
            let block =
                bitcoin_http_request_for_hex_encoded_object::<Self::Block>(url, client).await?;
            tracing::debug!(
                "Fetched block {} with {} transactions from bitcoind",
                block_hash,
                block.txdata.len()
            );

            Ok(block)
        }
        .boxed()
        .compat();

        Box::new(block)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::quickcheck::Quickcheck;
    use bitcoin::hashes::sha256d;

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
        fn prop(hash: Quickcheck<BlockHash>) -> bool {
            for base_url in base_urls() {
                let connector = BitcoindConnector::new(base_url, Network::Regtest).unwrap();

                connector.raw_block_by_hash_url(&hash);
            }

            true // not panicing is good enough for this test
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<BlockHash>) -> bool)
    }

    #[test]
    fn given_different_base_urls_correct_sub_urls_are_built() {
        for base_url in base_urls() {
            let connector = BitcoindConnector::new(base_url, Network::Regtest).unwrap();

            let chaininfo_url = connector.chaininfo_url.clone();
            assert_eq!(
                chaininfo_url,
                Url::parse("http://localhost:8080/rest/chaininfo.json").unwrap()
            );

            let block_id: sha256d::Hash =
                "2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02"
                    .parse()
                    .unwrap();
            let raw_block_by_hash_url = connector.raw_block_by_hash_url(&block_id.into());
            assert_eq!(raw_block_by_hash_url, Url::parse("http://localhost:8080/rest/block/2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02.hex").unwrap());
        }
    }
}
