use crate::jsonrpc;
use serde::Deserialize;

struct Client {
    rpc_client: jsonrpc::Client,
}

impl Client {
    pub fn new(url: reqwest::Url) -> Self {
        Client {
            rpc_client: jsonrpc::Client::new(url),
        }
    }

    pub async fn network(&self) -> anyhow::Result<String> {
        let blockchain_info = self
            .rpc_client
            .send::<Vec<()>, BlockchainInfo>(jsonrpc::Request::new("getblockchaininfo", vec![]))
            .await?;

        Ok(blockchain_info.chain)
    }
}

#[derive(Debug, Deserialize)]
struct BlockchainInfo {
    chain: String,
}

#[cfg(all(test, feature = "test-docker"))]
mod test {
    use super::*;
    use crate::test_harness::BitcoinBlockchain;
    use testcontainers::clients;

    #[tokio::test]
    async fn get_network_info() {
        let tc_client = clients::Cli::default();
        let blockchain = BitcoinBlockchain::new(&tc_client).unwrap();

        let client = Client::new(blockchain.node_url);

        let network = client.network().await.unwrap();

        assert_eq!(network.as_str(), "regtest")
    }
}
