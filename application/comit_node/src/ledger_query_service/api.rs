use failure::Error;
use reqwest::Url;
use std::{fmt::Debug, marker::PhantomData};
use swap_protocols::ledger::Ledger;

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

pub trait LedgerQueryServiceApiClient<L: Ledger, Q>: 'static + Debug + Send + Sync {
    fn create(&self, query: Q) -> Result<QueryId<L>, Error>;
    fn fetch_results(&self, query: &QueryId<L>) -> Result<Vec<L::TxId>, Error>;
    fn delete(&self, query: &QueryId<L>);
}
