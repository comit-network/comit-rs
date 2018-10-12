pub use self::{bitcoin::*, client::*, ethereum::*};
use reqwest::{self, Url, UrlError};
use std::marker::PhantomData;
use swap_protocols::ledger::Ledger;

mod bitcoin;
mod client;
mod ethereum;
pub mod fake_query_service;
pub mod fetch_transaction_stream;

#[derive(Clone, Debug)]
pub struct QueryId<L: Ledger> {
    location: Url,
    ledger_type: PhantomData<L>,
}

impl<L: Ledger> AsRef<Url> for QueryId<L> {
    fn as_ref(&self) -> &Url {
        &self.location
    }
}

impl<L: Ledger> QueryId<L> {
    pub fn new(location: Url) -> Self {
        QueryId {
            location,
            ledger_type: PhantomData,
        }
    }
}

#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "The provided endpoint was malformed.")]
    MalformedEndpoint(UrlError),
    #[fail(display = "The request failed to send.")]
    FailedRequest(reqwest::Error),
    #[fail(display = "The response did not contain a Location header.")]
    MissingLocation,
    #[fail(display = "The returned URL could not be parsed.")]
    MalformedLocation(#[cause] UrlError),
    #[fail(display = "The ledger is not support.")]
    UnsupportedLedger,
}

pub trait LedgerQueryServiceApiClient<L: Ledger, Q>: 'static + Send + Sync {
    fn create(&self, query: Q) -> Result<QueryId<L>, Error>;
    fn fetch_results(&self, query: &QueryId<L>) -> Result<Vec<L::TxId>, Error>;
    fn delete(&self, query: &QueryId<L>);
}
