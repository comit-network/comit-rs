use bitcoin_rpc::BitcoinCoreClient;

pub fn create_client() -> BitcoinCoreClient {
    let url = env!("BITCOIN_RPC_URL");
    let username = env!("BITCOIN_RPC_USERNAME");
    let password = env!("BITCOIN_RPC_PASSWORD");

    BitcoinCoreClient::new(url, username, password)
}
