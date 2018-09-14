use bitcoin_fee_service;
use bitcoin_rpc_client;
use bitcoin_support;
use event_store;
use reqwest;
use rocket::response::status::BadRequest;
use rustc_hex;
use swap_protocols::rfc003::ledger_htlc_service;

#[derive(Debug)] //TODO merge these errors into error
pub enum Error {
    ComitNode(reqwest::Error),
    TradingService(String), //TODO this should not exist anymore
    EventStore(event_store::Error),
    FeeService(bitcoin_fee_service::Error),
    LedgerHtlcService(ledger_htlc_service::Error),
    BitcoinRpc(bitcoin_rpc_client::RpcError),
    BitcoinNode(reqwest::Error),
    Unlocking(String),
}

impl From<Error> for BadRequest<String> {
    fn from(e: Error) -> Self {
        error!("{:?}", e);
        BadRequest(None)
    }
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

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        error!("{:?}", e);
        Error::EventStore(e)
    }
}

impl From<bitcoin_fee_service::Error> for Error {
    fn from(e: bitcoin_fee_service::Error) -> Self {
        error!("{:?}", e);
        Error::FeeService(e)
    }
}

impl From<bitcoin_rpc_client::RpcError> for Error {
    fn from(e: bitcoin_rpc_client::RpcError) -> Self {
        error!("{:?}", e);
        Error::BitcoinRpc(e)
    }
}

impl From<ledger_htlc_service::Error> for Error {
    fn from(e: ledger_htlc_service::Error) -> Self {
        Error::LedgerHtlcService(e)
    }
}
