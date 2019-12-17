use crate::btsieve::{
    bitcoin::bitcoin_http_request_for_hex_encoded_object, BlockByHash, BlockCache, LatestBlock,
};
use async_std::sync::Mutex;
use async_trait::async_trait;
use bitcoin::{hashes::sha256d::Hash, Block, Network};
use futures_core::{compat::Future01CompatExt, TryFutureExt};
use lru_cache::LruCache;
use reqwest::{r#async::Client, Url};
use serde::Deserialize;
use std::sync::Arc;
use tokio::prelude::Future;

#[derive(Deserialize)]
struct ChainInfo {
    bestblockhash: Hash,
}

#[derive(Clone, Debug)]
pub struct BitcoindConnector {
    chaininfo_url: Url,
    raw_block_by_hash_url: Url,
    client: Client,
    block_cache: BitcoindBlockCache,
}

impl BitcoindConnector {
    pub fn new(
        base_url: Url,
        _network: Network,
        cache_capacity: usize,
    ) -> Result<Self, reqwest::UrlError> {
        Ok(Self {
            chaininfo_url: base_url.join("rest/chaininfo.json")?,
            raw_block_by_hash_url: base_url.join("rest/block/")?,
            client: Client::new(),
            block_cache: BitcoindBlockCache::new(cache_capacity),
        })
    }

    fn raw_block_by_hash_url(&self, block_hash: &Hash) -> Url {
        self.raw_block_by_hash_url
            .join(&format!("{}.hex", block_hash))
            .expect("building url should work")
    }
}

#[derive(Clone, Debug)]
pub struct BitcoindBlockCache {
    map: Arc<Mutex<LruCache<Hash, Block>>>,
}

impl BitcoindBlockCache {
    fn new(capacity: usize) -> Self {
        let map: LruCache<Hash, Block> = LruCache::new(capacity);
        Self {
            map: Arc::new(Mutex::new(map)),
        }
    }
}

#[async_trait]
impl BlockCache for BitcoindBlockCache {
    type Block = Block;
    type BlockHash = Hash;

    async fn get(&self, block_hash: &Hash) -> anyhow::Result<Option<Block>> {
        let mut cache = self.map.lock().await;
        Ok(cache.get_mut(block_hash).cloned())
    }

    async fn insert(&mut self, block_hash: Hash, block: Block) -> anyhow::Result<Option<Block>> {
        let mut cache = self.map.lock().await;
        Ok(cache.insert(block_hash, block))
    }
}

impl LatestBlock for BitcoindConnector {
    type Error = crate::btsieve::bitcoin::Error;
    type Block = Block;
    type BlockHash = Hash;

    fn latest_block(
        &mut self,
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
    type Error = crate::btsieve::bitcoin::Error;
    type Block = Block;
    type BlockHash = Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(Box::pin(block_by_hash(self.clone(), block_hash)).compat())
    }
}

async fn block_by_hash(
    connector: BitcoindConnector,
    block_hash: Hash,
) -> Result<bitcoin::Block, crate::btsieve::bitcoin::Error> {
    let mut cache = connector.block_cache.clone();

    if let Ok(Some(block)) = cache.get(&block_hash).await {
        log::trace!("Found block in cache: {:?}", block);
        return Ok(block.clone());
    }

    let url = connector.raw_block_by_hash_url(&block_hash);

    let block = bitcoin_http_request_for_hex_encoded_object::<bitcoin::Block>(
        url,
        connector.client.clone(),
    )
    .compat()
    .await?;

    log::trace!("Fetched block from bitcoind: {:?}", block);

    let _ = cache.insert(block_hash, block.clone());
    Ok(block)
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
        fn prop(hash: Quickcheck<Hash>) -> bool {
            for base_url in base_urls() {
                let blocksource = BitcoindConnector::new(base_url, Network::Regtest).unwrap();

                blocksource.raw_block_by_hash_url(&hash);
            }

            true // not panicing is good enough for this test
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<Hash>) -> bool)
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
