use crate::btsieve::{
    bitcoin::bitcoin_http_request_for_hex_encoded_object, BlockByHash, LatestBlock,
};
use bitcoin::{hashes::sha256d, Network};
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Deserialize)]
struct BlockchainInfoLatestBlock {
    hash: sha256d::Hash,
}

#[derive(Clone, Debug)]
pub struct BlockchainInfoConnector {
    client: Client,
}

impl BlockchainInfoConnector {
    pub fn new(network: Network) -> Result<Self, crate::btsieve::bitcoin::Error> {
        // Currently configured for Mainnet only because blockchain.info does not
        // support hex-encoded block retrieval for testnet.

        if network != Network::Bitcoin {
            log::error!(
                "Network {} not supported for bitcoin http connector",
                network
            );
            return Err(crate::btsieve::bitcoin::Error::UnsupportedNetwork(format!(
                "Network {} currently not supported for bitcoin http plocksource",
                network
            )));
        }

        Ok(Self {
            client: Client::new(),
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

#[async_trait::async_trait]
impl LatestBlock for BlockchainInfoConnector {
    type Error = crate::btsieve::bitcoin::Error;
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    async fn latest_block(&mut self) -> Result<Self::Block, Self::Error> {
        let latest_block_url = "https://blockchain.info/latestblock";
        let this = self.clone();

        let blockchain_info_latest_block = this
            .client
            .get(latest_block_url)
            .send()
            .await?
            .json::<BlockchainInfoLatestBlock>()
            .await?;

        let block = this
            .block_by_hash(blockchain_info_latest_block.hash)
            .await?;

        Ok(block)
    }
}

#[async_trait::async_trait]
impl BlockByHash for BlockchainInfoConnector {
    type Error = crate::btsieve::bitcoin::Error;
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block, Self::Error> {
        let url = Self::block_by_hash_url(&block_hash);
        let block =
            bitcoin_http_request_for_hex_encoded_object::<Self::Block>(url, self.client.clone())
                .await?;

        log::trace!("Fetched block from blockchain.info: {:?}", block);

        Ok(block)
    }
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
