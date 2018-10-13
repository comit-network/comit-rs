pub use self::{bitcoin::*, client::*, ethereum::*};
use failure;
use reqwest::{self, Url};
use std::marker::PhantomData;
use swap_protocols::ledger::Ledger;
use tokio::prelude::Future;
use url::ParseError;

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
    MalformedEndpoint(#[cause] ParseError),
    #[fail(display = "The request failed to send.")]
    FailedRequest(#[cause] reqwest::Error),
    #[fail(display = "The response was somehow malformed.")]
    MalformedResponse(failure::Error),
    #[fail(display = "The ledger is not support.")]
    UnsupportedLedger,
}

pub trait LedgerQueryServiceApiClient<L: Ledger, Q>: 'static + Send + Sync {
    fn create(&self, query: Q) -> Box<Future<Item = QueryId<L>, Error = Error> + Send>;
    fn fetch_results(
        &self,
        query: &QueryId<L>,
    ) -> Box<Future<Item = Vec<L::TxId>, Error = Error> + Send>;
    fn delete(&self, query: &QueryId<L>) -> Box<Future<Item = (), Error = Error> + Send>;
}
