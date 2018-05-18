use bitcoin_rpc::BitcoinCoreClient;
use std::env::var;

pub fn create_client() -> BitcoinCoreClient {
    let url = var("BITCOIN_RPC_URL").unwrap();
    let username = var("BITCOIN_RPC_USERNAME").unwrap();
    let password = var("BITCOIN_RPC_PASSWORD").unwrap();

    BitcoinCoreClient::new(url.as_str(), username.as_str(), password.as_str())
}
