use crate::{
    btsieve::{BlockByHash, ConnectedNetwork, LatestBlock},
    ledger,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use bitcoin::{consensus::deserialize, BlockHash};
use reqwest::{Client, Url};
use serde::Deserialize;

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct ChainInfo {
    bestblockhash: BlockHash,
    #[serde(with = "ledger::bitcoin::bitcoind_jsonrpc_network")]
    pub chain: ledger::Bitcoin,
}

#[derive(Debug)]
pub struct BitcoindConnector {
    chaininfo_url: Url,
    raw_block_by_hash_url: Url,
    client: Client,
}

impl BitcoindConnector {
    pub fn new(base_url: Url) -> Result<Self> {
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

    pub async fn chain_info(&self) -> Result<ChainInfo> {
        let url = &self.chaininfo_url;
        let chain_info = self
            .client
            .get(url.clone())
            .send()
            .await
            .with_context(|| GetRequestFailed(url.clone()))?
            .json::<ChainInfo>()
            .await
            .context("failed to deserialize JSON response as chaininfo")?;

        Ok(chain_info)
    }
}

#[async_trait]
impl LatestBlock for BitcoindConnector {
    type Block = bitcoin::Block;

    async fn latest_block(&self) -> Result<Self::Block> {
        let chain_info = self.chain_info().await?;
        let block = self.block_by_hash(chain_info.bestblockhash).await?;

        Ok(block)
    }
}

#[async_trait]
impl BlockByHash for BitcoindConnector {
    type Block = bitcoin::Block;
    type BlockHash = bitcoin::BlockHash;

    async fn block_by_hash(&self, block_hash: Self::BlockHash) -> Result<Self::Block> {
        let url = self.raw_block_by_hash_url(&block_hash);
        let response = self
            .client
            .get(url.clone())
            .send()
            .await
            .with_context(|| GetRequestFailed(url.clone()))?
            .text()
            .await
            .with_context(|| format!("failed to read response body for GET {}", url))?;

        let block = decode_response(response)?;

        Ok(block)
    }
}

#[async_trait]
impl ConnectedNetwork for BitcoindConnector {
    type Network = ledger::Bitcoin;

    async fn connected_network(&self) -> anyhow::Result<ledger::Bitcoin> {
        let chain = self.chain_info().await?.chain;

        Ok(chain)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("GET request to {0} failed")]
pub struct GetRequestFailed(Url);

fn decode_response(response_text: String) -> Result<bitcoin::Block> {
    let bytes = hex::decode(response_text.trim()).context("failed to decode hex")?;
    let block = deserialize(bytes.as_slice()).context("failed to deserialize bytes as block")?;

    Ok(block)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::ledger::bitcoin::Bitcoin;
    use bitcoin::hashes::sha256d;
    use spectral::prelude::*;

    fn base_urls() -> Vec<Url> {
        vec![
            "http://localhost:8080".parse().unwrap(),
            "http://localhost:8080/".parse().unwrap(),
        ]
    }

    #[test]
    fn constructor_does_not_fail_for_base_urls() {
        for base_url in base_urls() {
            let result = BitcoindConnector::new(base_url);

            assert!(result.is_ok());
        }
    }

    #[test]
    fn given_different_base_urls_correct_sub_urls_are_built() {
        for base_url in base_urls() {
            let connector = BitcoindConnector::new(base_url).unwrap();

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
        assert_eq!(info.chain, Bitcoin::Testnet);

        let chain_info = r#"{
    "chain": "main",
    "bestblockhash": "00000000000000c473d592c8637824b8362d522af18bfb1d0e92107b96ecdc5c"
  }
  "#;
        let info = serde_json::from_str::<ChainInfo>(chain_info).unwrap();
        assert_eq!(info.chain, Bitcoin::Mainnet);

        let chain_info = r#"{
    "chain": "regtest",
    "bestblockhash": "00000000000000c473d592c8637824b8362d522af18bfb1d0e92107b96ecdc5c"
  }
  "#;
        let info = serde_json::from_str::<ChainInfo>(chain_info).unwrap();
        assert_eq!(info.chain, Bitcoin::Regtest);
    }

    #[test]
    fn can_decode_block_from_bitcoind_http_interface() {
        // the line break here is on purpose, as it is returned like that from bitcoind
        let block = r#"00000020837603de6069115e22e7fbf063c2a6e3bc3b3206f0b7e08d6ab6c168c2e50d4a9b48676dedc93d05f677778c1d83df28fd38d377548340052823616837666fb8be1b795dffff7f200000000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401650101ffffffff0200f2052a0100000023210205980e76eee77386241a3a7a5af65e910fb7be411b98e609f7c0d97c50ab8ebeac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000
"#.to_owned();

        let bytes = decode_response(block);

        assert_that(&bytes).is_ok();
    }
}
