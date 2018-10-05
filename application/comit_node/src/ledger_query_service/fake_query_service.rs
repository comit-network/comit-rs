use failure::Error;
use ledger_query_service::{LedgerQueryServiceApiClient, QueryId};
use std::sync::Mutex;
use swap_protocols::ledger::Ledger;

#[derive(Debug)]
pub struct SimpleFakeLedgerQueryService<L: Ledger> {
    pub results: Vec<L::TxId>,
}

impl<L: Ledger, Q> LedgerQueryServiceApiClient<L, Q> for SimpleFakeLedgerQueryService<L> {
    fn create(&self, _query: Q) -> Result<QueryId<L>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<L>) -> Result<Vec<L::TxId>, Error> {
        Ok(self.results.clone())
    }

    fn delete(&self, _query: &QueryId<L>) {
        unimplemented!()
    }
}

#[derive(Debug)]
pub struct InvocationCountFakeLedgerQueryService<L: Ledger> {
    pub number_of_invocations_before_result: u32,
    pub invocations: Mutex<u32>,
    pub results: Vec<L::TxId>,
}

impl<L: Ledger, Q> LedgerQueryServiceApiClient<L, Q> for InvocationCountFakeLedgerQueryService<L> {
    fn create(&self, _query: Q) -> Result<QueryId<L>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<L>) -> Result<Vec<L::TxId>, Error> {
        let mut invocations = self.invocations.lock().unwrap();

        *invocations += 1;

        if *invocations >= self.number_of_invocations_before_result {
            Ok(self.results.clone())
        } else {
            Ok(Vec::new())
        }
    }

    fn delete(&self, _query: &QueryId<L>) {
        unimplemented!()
    }
}
