use crate::btsieve::{
    bitcoin::bitcoin_http_request_for_hex_encoded_object, BlockByHash, LatestBlock,
};
use bitcoin::{hashes::sha256d, Network};
use futures_core::{compat::Future01CompatExt, TryFutureExt};
use reqwest::{r#async::Client, Url};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::prelude::Future;

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: sha256d::Hash,
}

#[derive(Clone, Debug)]
pub struct BlockchainInfoConnector {
    client: Client,
    block_cache: HashMap<sha256d::Hash, bitcoin::Block>,
}

impl BlockchainInfoConnector {
    pub fn new(network: Network) -> Result<Self, crate::btsieve::bitcoin::Error> {
        // Currently configured for Mainnet only because blockchain.info does not
        // support hex-encoded block retrieval for testnet.

        if network != Network::Bitcoin {
            log::error!(
                "Network {} not supported for bitcoin http blocksource",
                network
            );
            return Err(crate::btsieve::bitcoin::Error::UnsupportedNetwork(format!(
                "Network {} currently not supported for bitcoin http plocksource",
                network
            )));
        }

        let block_cache: HashMap<sha256d::Hash, bitcoin::Block> = HashMap::new();

        Ok(Self {
            client: Client::new(),
            block_cache,
        })
    }

    fn block_by_hash_url(block_hash: &sha256d::Hash) -> Url {
        let block_hash = block_hash.to_string();
        let mut url = Url::parse("https://blockchain.info/rawblock/")
            .unwrap()
            .join(&block_hash)
            .unwrap();
        url.set_query(Some("format=hex"));

        url
    }
}

impl LatestBlock for BlockchainInfoConnector {
    type Error = crate::btsieve::bitcoin::Error;
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        let latest_block_url = "https://blockchain.info/latestblock";
        let latest_block_without_tx = self
            .client
            .get(latest_block_url)
            .send()
            .map_err(crate::btsieve::bitcoin::Error::Reqwest)
            .and_then(move |mut response| {
                response
                    .json::<BlockchainInfoLatestBlock>()
                    .map_err(crate::btsieve::bitcoin::Error::Reqwest)
            });

        let mut cloned_self = self.clone();

        Box::new(
            latest_block_without_tx
                .and_then(move |latest_block| cloned_self.block_by_hash(latest_block.hash)),
        )
    }
}

impl BlockByHash for BlockchainInfoConnector {
    type Error = crate::btsieve::bitcoin::Error;
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn block_by_hash(
        &mut self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(Box::pin(block_by_hash(self.clone(), block_hash)).compat())
    }
}

async fn block_by_hash(
    mut connector: BlockchainInfoConnector,
    block_hash: sha256d::Hash,
) -> Result<bitcoin::Block, crate::btsieve::bitcoin::Error> {
    if let Some(block) = connector.block_cache.get(&block_hash) {
        log::trace!("Found block in cache: {:?}", block);
        return Ok(block.clone());
    }

    let block = bitcoin_http_request_for_hex_encoded_object::<bitcoin::Block>(
        BlockchainInfoConnector::block_by_hash_url(&block_hash),
        connector.client.clone(),
    )
    .compat()
    .await?;

    log::trace!("Fetched block from blockchain.info: {:?}", block);

    connector.block_cache.insert(block_hash, block.clone());
    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quickcheck::Quickcheck;
    use std::str::FromStr;

    #[test]
    fn block_by_hash_url_never_panics() {
        fn prop(hash: Quickcheck<sha256d::Hash>) -> bool {
            BlockchainInfoConnector::block_by_hash_url(&hash);

            true
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<sha256d::Hash>) -> bool)
    }

    #[test]
    fn block_by_hash_url_creates_correct_url() {
        let block_id = sha256d::Hash::from_str(
            "2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02",
        )
        .unwrap();
        let actual_url = BlockchainInfoConnector::block_by_hash_url(&block_id);

        let expected_url = Url::parse("https://blockchain.info/rawblock/2a593b84b1943521be01f97a59fc7feba30e7e8527fb2ba20b0158ca09016d02?format=hex").unwrap();

        assert_eq!(actual_url, expected_url);
    }
}
