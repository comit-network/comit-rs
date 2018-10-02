use bitcoin_support::TransactionId;
use failure::Error;
use ledger_query_service::{
    api::{LedgerQueryServiceApiClient, QueryId},
    bitcoin::BitcoinQuery,
};
use std::sync::Mutex;
use swap_protocols::ledger::bitcoin::Bitcoin;

pub struct SimpleFakeLedgerQueryService {
    pub results: Vec<TransactionId>,
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for SimpleFakeLedgerQueryService {
    fn create(&self, _query: BitcoinQuery) -> Result<QueryId<Bitcoin>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<Bitcoin>) -> Result<Vec<TransactionId>, Error> {
        Ok(self.results.clone())
    }

    fn delete(&self, _query: &QueryId<Bitcoin>) {
        unimplemented!()
    }
}

pub struct InvocationCountFakeLedgerQueryService {
    pub number_of_invocations_before_result: u32,
    pub invocations: Mutex<u32>,
    pub results: Vec<TransactionId>,
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for InvocationCountFakeLedgerQueryService {
    fn create(&self, _query: BitcoinQuery) -> Result<QueryId<Bitcoin>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<Bitcoin>) -> Result<Vec<TransactionId>, Error> {
        let mut invocations = self.invocations.lock().unwrap();

        *invocations += 1;

        if *invocations >= self.number_of_invocations_before_result {
            Ok(self.results.clone())
        } else {
            Ok(Vec::new())
        }
    }

    fn delete(&self, _query: &QueryId<Bitcoin>) {
        unimplemented!()
    }
}
