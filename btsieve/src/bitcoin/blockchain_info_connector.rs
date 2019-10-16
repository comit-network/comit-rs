use crate::{
    bitcoin::{self, bitcoin_http_request_for_hex_encoded_object},
    BlockByHash, LatestBlock,
};
use bitcoin_support::Network;
use reqwest::{r#async::Client, Url};
use serde::Deserialize;
use tokio::prelude::Future;

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: bitcoin_support::BlockId,
}

#[derive(Clone)]
pub struct BlockchainInfoConnector {
    client: Client,
}

impl BlockchainInfoConnector {
    pub fn new(network: Network) -> Result<Self, bitcoin::Error> {
        // Currently configured for Mainnet only because blockchain.info does not
        // support hex-encoded block retrieval for testnet.

        if network != Network::Mainnet {
            log::error!(
                "Network {} not supported for bitcoin http blocksource",
                network
            );
            return Err(bitcoin::Error::UnsupportedNetwork(format!(
                "Network {} currently not supported for bitcoin http plocksource",
                network
            )));
        }

        Ok(Self {
            client: Client::new(),
        })
    }

    fn block_by_hash_url(block_hash: &bitcoin_support::BlockId) -> Url {
        let block_hash = format!("{}", block_hash);
        let mut url = Url::parse("https://blockchain.info/rawblock/")
            .unwrap()
            .join(&block_hash)
            .unwrap();
        url.set_query(Some("format=hex"));

        url
    }
}

impl LatestBlock for BlockchainInfoConnector {
    type Error = bitcoin::Error;
    type Block = bitcoin_support::Block;
    type BlockHash = bitcoin_support::BlockId;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let latest_block_url = "https://blockchain.info/latestblock";
        let latest_block_without_tx = self
            .client
            .get(latest_block_url)
            .send()
            .map_err(bitcoin::Error::Reqwest)
            .and_then(move |mut response| {
                response
                    .json::<BlockchainInfoLatestBlock>()
                    .map_err(bitcoin::Error::Reqwest)
            });

        let cloned_self = self.clone();

        Box::new(
            latest_block_without_tx
                .and_then(move |latest_block| cloned_self.block_by_hash(latest_block.hash)),
        )
    }
}

impl BlockByHash for BlockchainInfoConnector {
    type Error = bitcoin::Error;
    type Block = bitcoin_support::Block;
    type BlockHash = bitcoin_support::BlockId;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let block = bitcoin_http_request_for_hex_encoded_object::<Self::Block>(
            Self::block_by_hash_url(&block_hash),
            self.client.clone(),
        );

        Box::new(block.inspect(|block| {
            log::trace!("Fetched block from blockchain.info: {:?}", block);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quickcheck::Quickcheck;
    use std::str::FromStr;

    #[test]
    fn block_by_hash_url_never_panics() {
        fn prop(hash: Quickcheck<bitcoin_support::BlockId>) -> bool {
            BlockchainInfoConnector::block_by_hash_url(&hash);

            true
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<bitcoin_support::BlockId>) -> bool)
    }

    #[test]
    fn block_by_hash_url_creates_correct_url() {
        let block_id = bitcoin_support::BlockId::from_str(
            "2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02",
        )
        .unwrap();
        let actual_url = BlockchainInfoConnector::block_by_hash_url(&block_id);

        let expected_url = Url::parse("https://blockchain.info/rawblock/2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02?format=hex").unwrap();

        assert_eq!(actual_url, expected_url);
    }
}
