use crate::jsonrpc;
use bitcoin::Address;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Client {
    rpc_client: jsonrpc::Client,
}

impl Client {
    pub fn new(url: reqwest::Url) -> Self {
        dbg!(&url);
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

    pub async fn create_wallet(
        &self,
        wallet_name: &str,
        disable_private_keys: Option<bool>,
        blank: Option<bool>,
        passphrase: String,
        avoid_reuse: Option<bool>,
    ) -> anyhow::Result<CreateWalletResponse> {
        let response = self
            .rpc_client
            .send(jsonrpc::Request::new(
                "createwallet",
                vec![
                    jsonrpc::serialize(wallet_name)?,
                    jsonrpc::serialize(disable_private_keys)?,
                    jsonrpc::serialize(blank)?,
                    jsonrpc::serialize(passphrase)?,
                    jsonrpc::serialize(avoid_reuse)?,
                ],
            ))
            .await?;
        Ok(response)
    }

    pub async fn set_hd_seed(
        &self,
        wallet_name: &str,
        new_key_pool: Option<bool>,
        wif_private_key: Option<String>,
    ) -> anyhow::Result<()> {
        self.rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "sethdseed",
                    vec![
                        jsonrpc::serialize(new_key_pool)?,
                        jsonrpc::serialize(wif_private_key)?,
                    ],
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn get_new_address(
        &self,
        wallet_name: &str,
        label: Option<String>,
        address_type: Option<String>,
    ) -> anyhow::Result<Address> {
        let address = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "getnewaddress",
                    vec![
                        jsonrpc::serialize(label)?,
                        jsonrpc::serialize(address_type)?,
                    ],
                ),
            )
            .await?;
        Ok(address)
    }

    #[cfg(test)]
    pub async fn generate_to_address(
        &self,
        nblocks: u32,
        address: Address,
        max_tries: Option<u32>,
    ) -> anyhow::Result<Vec<BlockHash>> {
        let response = self
            .rpc_client
            .send(jsonrpc::Request::new(
                "generatetoaddress",
                vec![
                    jsonrpc::serialize(nblocks)?,
                    jsonrpc::serialize(address)?,
                    jsonrpc::serialize(max_tries)?,
                ],
            ))
            .await?;
        Ok(response)
    }
}

#[derive(Debug, Deserialize)]
struct BlockchainInfo {
    chain: String,
}

#[derive(Debug, Deserialize)]
pub struct BlockHash(String);

#[derive(Debug, Deserialize)]
pub struct CreateWalletResponse {
    name: String,
    warning: String,
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
