use crate::{
    btsieve::{bitcoin::bitcoin_http_request_for_hex_encoded_object, BlockByHash, LatestBlock},
    config::validation::FetchNetworkId,
};
use async_trait::async_trait;
use bitcoin::{BlockHash, Network};
use reqwest::{Client, Url};
use serde::{de, export::fmt, Deserialize, Deserializer};

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct ChainInfo {
    bestblockhash: BlockHash,
    #[serde(deserialize_with = "deserialize_bitcoind_values")]
    pub chain: Network,
}

#[derive(Debug)]
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

#[async_trait]
impl LatestBlock for BitcoindConnector {
    type Block = bitcoin::Block;

    async fn latest_block(&self) -> anyhow::Result<Self::Block> {
        let chaininfo_url = self.chaininfo_url.clone();

        let chain_info = self
            .client
            .get(chaininfo_url)
            .send()
            .await?
            .json::<ChainInfo>()
            .await?;

        let block = self.block_by_hash(chain_info.bestblockhash).await?;

        Ok(block)
    }
}

#[async_trait]
impl BlockByHash for BitcoindConnector {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> anyhow::Result<Self::Block> {
        let url = self.raw_block_by_hash_url(&block_hash);
        let block =
            bitcoin_http_request_for_hex_encoded_object::<Self::Block>(url, &self.client).await?;

        tracing::debug!(
            "Fetched block {} with {} transactions from bitcoind",
            block_hash,
            block.txdata.len()
        );

        Ok(block)
    }
}

#[async_trait]
impl FetchNetworkId<Network> for BitcoindConnector {
    async fn network_id(&self) -> anyhow::Result<Network> {
        let client = self.client.clone();
        let chaininfo_url = self.chaininfo_url.clone();

        let chain_info: ChainInfo = client
            .get(chaininfo_url)
            .send()
            .await?
            .json::<ChainInfo>()
            .await?;

        tracing::debug!("Fetched chain info: {:?} from bitcoind", chain_info);

        Ok(chain_info.chain)
    }
}

pub fn deserialize_bitcoind_values<'de, D>(deserializer: D) -> Result<bitcoin::Network, D::Error>
where
    D: Deserializer<'de>,
{
    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = bitcoin::Network;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bitcoin network")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            match v {
                "main" => Ok(bitcoin::Network::Bitcoin),
                "test" => Ok(bitcoin::Network::Testnet),
                "regtest" => Ok(bitcoin::Network::Regtest),
                unknown => Err(E::custom(format!("unknown bitcoin network {}", unknown))),
            }
        }
    }

    deserializer.deserialize_str(Visitor)
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

    #[test]
    fn test_custom_serde_deserializer() {
        let chain_info = r#"{
    "chain": "test",
    "bestblockhash": "00000000000000c473d592c8637824b8362d522af18bfb1d0e92107b96ecdc5c"
  }
  "#;
        let info = serde_json::from_str::<ChainInfo>(chain_info).unwrap();
        assert_eq!(info.chain, Network::Testnet);

        let chain_info = r#"{
    "chain": "main",
    "bestblockhash": "00000000000000c473d592c8637824b8362d522af18bfb1d0e92107b96ecdc5c"
  }
  "#;
        let info = serde_json::from_str::<ChainInfo>(chain_info).unwrap();
        assert_eq!(info.chain, Network::Bitcoin);

        let chain_info = r#"{
    "chain": "regtest",
    "bestblockhash": "00000000000000c473d592c8637824b8362d522af18bfb1d0e92107b96ecdc5c"
  }
  "#;
        let info = serde_json::from_str::<ChainInfo>(chain_info).unwrap();
        assert_eq!(info.chain, Network::Regtest);
    }
}
