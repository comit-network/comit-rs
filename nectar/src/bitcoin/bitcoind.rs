use crate::{
    bitcoin::{Address, Amount, Network},
    jsonrpc,
};
use ::bitcoin::{consensus::encode::serialize_hex, hashes::hex::FromHex, Transaction, Txid};
use anyhow::Context;
use bitcoin::OutPoint;
use serde::Deserialize;

pub const JSONRPC_VERSION: &str = "1.0";

#[derive(Debug, Clone)]
pub struct Client {
    rpc_client: jsonrpc::Client,
}

impl Client {
    pub fn new(url: url::Url) -> Self {
        Client {
            rpc_client: jsonrpc::Client::new(url),
        }
    }

    pub async fn network(&self) -> anyhow::Result<Network> {
        let blockchain_info = self
            .rpc_client
            .send::<Vec<()>, BlockchainInfo>(jsonrpc::Request::new(
                "getblockchaininfo",
                vec![],
                JSONRPC_VERSION.into(),
            ))
            .await?;

        Ok(blockchain_info.chain)
    }

    pub async fn create_wallet(
        &self,
        wallet_name: &str,
        disable_private_keys: Option<bool>,
        blank: Option<bool>,
        passphrase: Option<String>,
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
                JSONRPC_VERSION.into(),
            ))
            .await
            .context("failed to create wallet")?;
        Ok(response)
    }

    pub async fn rescan(&self, wallet_name: &str) -> anyhow::Result<RescanResponse> {
        let response = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "rescanblockchain",
                    Vec::<serde_json::Value>::new(),
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to rescan")?;

        Ok(response)
    }

    pub async fn get_balance(
        &self,
        wallet_name: &str,
        minimum_confirmation: Option<u32>,
        include_watch_only: Option<bool>,
        avoid_reuse: Option<bool>,
    ) -> anyhow::Result<Amount> {
        let response = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "getbalance",
                    vec![
                        jsonrpc::serialize('*')?,
                        jsonrpc::serialize(minimum_confirmation)?,
                        jsonrpc::serialize(include_watch_only)?,
                        jsonrpc::serialize(avoid_reuse)?,
                    ],
                    JSONRPC_VERSION.into(),
                ),
            )
            .await?;
        let amount = Amount::from_btc(response)?;
        Ok(amount)
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
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to set HD seed")?;

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
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to get new address")?;
        Ok(address)
    }

    pub async fn get_wallet_info(&self, wallet_name: &str) -> anyhow::Result<WalletInfoResponse> {
        let response = self
            .rpc_client
            .send_with_path::<Vec<()>, _>(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new("getwalletinfo", vec![], JSONRPC_VERSION.into()),
            )
            .await?;
        Ok(response)
    }

    pub async fn send_to_address(
        &self,
        wallet_name: &str,
        address: Address,
        amount: Amount,
    ) -> anyhow::Result<Txid> {
        let txid: String = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "sendtoaddress",
                    vec![
                        jsonrpc::serialize(address)?,
                        jsonrpc::serialize(amount.as_btc())?,
                    ],
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to send to address")?;
        let txid = Txid::from_hex(&txid)?;

        Ok(txid)
    }

    pub async fn fund_htlc(
        &self,
        wallet_name: &str,
        address: Address,
        amount: Amount,
    ) -> anyhow::Result<OutPoint> {
        let address = address.to_string();

        let response: CreatePsbtResponse = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "walletcreatefundedpsbt",
                    serde_json::json! (
                        [
                            [],
                            [
                                {
                                    address: amount.as_btc()
                                }
                            ],
                            null,
                            {
                                "changePosition": 1 // this allows us to assume that the HTLC will always be at output position 0
                            }
                        ]
                    ),
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed create funded psbt")?;

        let response: ProcessPsbtResponse = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "walletprocesspsbt",
                    serde_json::json!([
                        response.psbt,
                        true,  // sign,
                        "ALL", // sighashtype,
                    ]),
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed process psbt")?;

        let response: FinalizePsbtResponse = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "finalizepsbt",
                    serde_json::json!([
                        response.psbt,
                        true, // extract,
                    ]),
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed finalize psbt")?;

        if !response.complete {
            anyhow::bail!("failed to finalize psbt")
        }

        let txid: String = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "sendrawtransaction",
                    vec![response.hex.expect("to be set if response.complete = true")],
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to send raw transaction")?;

        let txid = Txid::from_hex(&txid)?;

        Ok(OutPoint {
            txid,
            vout: 0, // we always put the change output on index 1, hence this must be 0
        })
    }

    pub async fn send_raw_transaction(
        &self,
        wallet_name: &str,
        transaction: Transaction,
    ) -> anyhow::Result<Txid> {
        let txid: String = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "sendrawtransaction",
                    vec![serialize_hex(&transaction)],
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to send raw transaction")?;
        let txid = Txid::from_hex(&txid)?;
        Ok(txid)
    }

    #[cfg(test)]
    pub async fn dump_wallet(
        &self,
        wallet_name: &str,
        filename: &std::path::Path,
    ) -> anyhow::Result<()> {
        let _: DumpWalletResponse = self
            .rpc_client
            .send_with_path(
                format!("/wallet/{}", wallet_name),
                jsonrpc::Request::new(
                    "dumpwallet",
                    vec![jsonrpc::serialize(filename)?],
                    JSONRPC_VERSION.into(),
                ),
            )
            .await
            .context("failed to dump wallet")?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn list_wallets(&self) -> anyhow::Result<Vec<String>> {
        let wallets: Vec<String> = self
            .rpc_client
            .send::<Vec<()>, _>(jsonrpc::Request::new(
                "listwallets",
                vec![],
                JSONRPC_VERSION.into(),
            ))
            .await
            .context("failed to list wallets")?;
        Ok(wallets)
    }

    #[allow(dead_code)]
    pub async fn derive_addresses(
        &self,
        descriptor: &str,
        range: Option<[u64; 2]>,
    ) -> anyhow::Result<Vec<Address>> {
        let addresses: Vec<Address> = self
            .rpc_client
            .send(jsonrpc::Request::new(
                "deriveaddresses",
                vec![jsonrpc::serialize(descriptor)?, jsonrpc::serialize(range)?],
                JSONRPC_VERSION.into(),
            ))
            .await
            .context("failed to derive addresses")?;
        Ok(addresses)
    }

    pub async fn get_descriptor_info(
        &self,
        descriptor: &str,
    ) -> anyhow::Result<GetDescriptorInfoResponse> {
        self.rpc_client
            .send(jsonrpc::Request::new(
                "getdescriptorinfo",
                vec![jsonrpc::serialize(descriptor)?],
                JSONRPC_VERSION.into(),
            ))
            .await
            .context("failed to get descriptor info")
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
                JSONRPC_VERSION.into(),
            ))
            .await
            .context("failed to generate to address")?;
        Ok(response)
    }
}

#[derive(Debug, Deserialize)]
struct BlockchainInfo {
    chain: Network,
}

#[derive(Debug, Deserialize)]
pub struct BlockHash(String);

#[derive(Debug, Deserialize)]
pub struct CreateWalletResponse {
    name: String,
    warning: String,
}

#[derive(Debug, Deserialize)]
pub struct RescanResponse {
    start_height: usize,
    stop_height: usize,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct WalletInfoResponse {
    #[serde(rename = "walletname")]
    pub wallet_name: String,
    #[serde(rename = "walletversion")]
    pub wallet_version: u32,
    #[serde(rename = "txcount")]
    pub tx_count: u32,
    #[serde(rename = "keypoololdest")]
    pub keypool_oldest: u32,
    #[serde(rename = "keypoolsize_hd_internal")]
    pub keypool_size_hd_internal: u32,
    pub unlocked_until: Option<u32>,
    #[serde(rename = "paytxfee")]
    pub pay_tx_fee: f64,
    #[serde(rename = "hdseedid")]
    pub hd_seed_id: Option<String>, // Hash 160
    pub private_keys_enabled: bool,
    pub avoid_reuse: bool,
    pub scanning: ScanProgress,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct DumpWalletResponse {
    filename: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GetDescriptorInfoResponse {
    pub descriptor: String,
    pub checksum: String,
    #[serde(rename = "isrange")]
    pub is_range: bool,
    #[serde(rename = "issolvable")]
    pub is_solvable: bool,
    #[serde(rename = "hasprivatekeys")]
    pub has_private_keys: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ScanProgress {
    Bool(bool),
    Progress { duration: u32, progress: f64 },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct CreatePsbtResponse {
    psbt: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ProcessPsbtResponse {
    psbt: String,
    complete: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct FinalizePsbtResponse {
    psbt: Option<String>,
    hex: Option<String>,
    complete: bool,
}

#[cfg(all(test, feature = "test-docker"))]
mod test {
    use super::*;
    use crate::test_harness::bitcoin;
    use testcontainers::clients;

    #[tokio::test]
    async fn get_network_info() {
        let client = {
            let tc_client = clients::Cli::default();
            let blockchain = bitcoin::Blockchain::new(&tc_client).unwrap();

            Client::new(blockchain.node_url)
        };

        let network = client.network().await.unwrap();

        assert_eq!(network, Network::Regtest)
    }

    #[test]
    fn decode_wallet_info() {
        let json = r#"{
        "walletname":"nectar_7426b018",
        "walletversion":169900,
        "balance":0.00000000,
        "unconfirmed_balance":0.00000000,
        "immature_balance":0.00000000,
        "txcount":0,
        "keypoololdest":1592792998,
        "keypoolsize":1000,
        "keypoolsize_hd_internal":1000,
        "paytxfee":0.00000000,
        "hdseedid":"4959e065fd8e278e4ffe62254897ddac18b02674",
        "private_keys_enabled":true,
        "avoid_reuse":false,
        "scanning":false
        }"#;

        let info: WalletInfoResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(info, WalletInfoResponse {
            wallet_name: "nectar_7426b018".into(),
            wallet_version: 169_900,
            tx_count: 0,
            keypool_oldest: 1_592_792_998,
            keypool_size_hd_internal: 1000,
            unlocked_until: None,
            pay_tx_fee: 0.0,
            hd_seed_id: Some("4959e065fd8e278e4ffe62254897ddac18b02674".into()),
            private_keys_enabled: true,
            avoid_reuse: false,
            scanning: ScanProgress::Bool(false)
        })
    }
}
