use bitcoin_support::TransactionId as BitcoinTxId;
use ethereum_support::H256 as EthereumTxId;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, Error, LedgerQueryServiceApiClient, QueryId,
};
use std::{fmt, marker::PhantomData, sync::Mutex};
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

#[derive(Debug)]
pub struct LedgerQueryServiceMock<L: Ledger, Q> {
    number_of_invocations: Mutex<u32>,
    results_for_next_invocation: Mutex<Option<Vec<L::TxId>>>,
    query_type: PhantomData<Q>,
}

impl<L: Ledger, Q> Default for LedgerQueryServiceMock<L, Q> {
    fn default() -> Self {
        Self {
            number_of_invocations: Mutex::new(0),
            results_for_next_invocation: Mutex::new(None),
            query_type: PhantomData,
        }
    }
}

impl<L: Ledger, Q> LedgerQueryServiceMock<L, Q> {
    pub fn set_next_results(&self, transactions: Vec<L::TxId>) {
        let mut results = self.results_for_next_invocation.lock().unwrap();

        *results = Some(transactions);
    }

    pub fn number_of_invocations(&self) -> u32 {
        *self.number_of_invocations.lock().unwrap()
    }
}

impl<L: Ledger, Q: fmt::Debug + Send + Sync + 'static> LedgerQueryServiceApiClient<L, Q>
    for LedgerQueryServiceMock<L, Q>
{
    fn create(&self, _query: Q) -> Result<QueryId<L>, Error> {
        Ok(QueryId::new("http://localhost/results/1".parse().unwrap()))
    }

    fn fetch_results(&self, _query: &QueryId<L>) -> Result<Vec<<L as Ledger>::TxId>, Error> {
        let mut invocations = self.number_of_invocations.lock().unwrap();

        let mut results = self.results_for_next_invocation.lock().unwrap();

        let transactions = results.take().unwrap_or(Vec::new());
        *invocations = *invocations + 1;

        Ok(transactions)
    }

    fn delete(&self, _query: &QueryId<L>) {
        unimplemented!()
    }
}
