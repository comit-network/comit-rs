pub use self::{bitcoin::*, cache::*, client::*, ethereum::*};
use reqwest::Url;
use std::{fmt::Debug, hash::Hash, marker::PhantomData};
use swap_protocols::ledger::Ledger;
use tokio::prelude::Future;

mod bitcoin;
mod cache;
mod client;
mod ethereum;
pub mod fake_query_service;
pub mod fetch_transaction_stream;

#[derive(Clone, Debug, PartialOrd, PartialEq)]
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

#[derive(Fail, Debug, Clone)]
pub enum Error {
    #[fail(display = "The request failed to send.")]
    FailedRequest,
    #[fail(display = "The response was somehow malformed.")]
    MalformedResponse,
}

pub trait Query: Sized + Clone + Debug + Send + Sync + Eq + Hash + 'static {}

pub trait LedgerQueryServiceApiClient<L: Ledger, Q: Query>:
    'static + Send + Sync + CreateQuery<L, Q> + FetchQueryResults<L>
{
    fn delete(&self, query: &QueryId<L>) -> Box<Future<Item = (), Error = Error> + Send>;
}

pub trait CreateQuery<L: Ledger, Q: Query>: 'static + Send + Sync {
    fn create_query(
        &self,
        query: Q,
    ) -> Box<Future<Item = QueryId<L>, Error = Error> + Send + 'static>;
}

pub trait FetchQueryResults<L: Ledger>: 'static + Send + Sync {
    fn fetch_query_results(
        &self,
        query: &QueryId<L>,
    ) -> Box<Future<Item = Vec<L::TxId>, Error = Error> + Send>;
}
