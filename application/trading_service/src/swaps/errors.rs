use bitcoin_support;
use event_store;
use reqwest;
use rocket::response::status::BadRequest;
use rustc_hex;

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ExchangeService(reqwest::Error),
    TradingService(String),
}

impl From<Error> for BadRequest<String> {
    fn from(e: Error) -> Self {
        error!("{:?}", e);
        BadRequest(None)
    }
}

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        Error::EventStore(e)
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
