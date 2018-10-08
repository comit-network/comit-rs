use bitcoin_support::TransactionId as BitcoinTxId;
use ethereum_support::H256 as EthereumTxId;
use failure::Error;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, LedgerQueryServiceApiClient, QueryId,
};
use std::sync::Mutex;
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};

#[derive(Debug)]
pub struct SimpleFakeLedgerQueryService {
    pub bitcoin_results: Vec<BitcoinTxId>,
    pub ethereum_results: Vec<EthereumTxId>,
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for SimpleFakeLedgerQueryService {
    fn create(&self, _query: BitcoinQuery) -> Result<QueryId<Bitcoin>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<Bitcoin>) -> Result<Vec<BitcoinTxId>, Error> {
        Ok(self.bitcoin_results.clone())
    }

    fn delete(&self, _query: &QueryId<Bitcoin>) {}
}

impl LedgerQueryServiceApiClient<Ethereum, EthereumQuery> for SimpleFakeLedgerQueryService {
    fn create(&self, _query: EthereumQuery) -> Result<QueryId<Ethereum>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<Ethereum>) -> Result<Vec<EthereumTxId>, Error> {
        Ok(self.ethereum_results.clone())
    }

    fn delete(&self, _query: &QueryId<Ethereum>) {}
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
