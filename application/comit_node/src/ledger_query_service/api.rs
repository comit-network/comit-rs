use failure::Error;
use ganp::ledger::Ledger;
use reqwest::Url;
use std::marker::PhantomData;

#[allow(dead_code)]
#[derive(Clone)]
pub struct QueryId<L: Ledger> {
    location: Url,
    ledger_type: PhantomData<L>,
}

impl<L: Ledger> QueryId<L> {
    pub fn new(location: Url) -> Self {
        QueryId {
            location,
            ledger_type: PhantomData,
        }
    }
}

pub trait LedgerQueryServiceApiClient<L: Ledger, Q>: Send + Sync {
    fn create(&self, query: Q) -> Result<QueryId<L>, Error>;
    fn fetch_results(&self, query: &QueryId<L>) -> Result<Vec<L::TxId>, Error>;
    fn delete(&self, query: &QueryId<L>);
}
