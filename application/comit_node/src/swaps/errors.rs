use bitcoin_rpc_client;
use bitcoin_support;
use reqwest;
use rustc_hex;

#[derive(Debug)] //TODO merge these errors into error
pub enum Error {
    TradingService(String), //TODO this should not exist anymore
    BitcoinRpc(bitcoin_rpc_client::RpcError),
    BitcoinNode(reqwest::Error),
    Unlocking(String),
}

impl From<bitcoin_support::serialize::Error> for Error {
    fn from(e: bitcoin_support::serialize::Error) -> Self {
        error!("Invalid bitcoin address format: {}", e);
        Error::TradingService(String::from("Invalid bitcoin address format"))
    }
}

impl From<rustc_hex::FromHexError> for Error {
    fn from(e: rustc_hex::FromHexError) -> Self {
        error!("Invalid ethereum address format: {}", e);
        Error::TradingService(String::from("Invalid ethereum address format"))
    }
}

impl From<bitcoin_rpc_client::RpcError> for Error {
    fn from(e: bitcoin_rpc_client::RpcError) -> Self {
        error!("{:?}", e);
        Error::BitcoinRpc(e)
    }
}
