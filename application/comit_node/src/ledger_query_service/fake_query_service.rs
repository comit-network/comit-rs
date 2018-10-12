use bitcoin_support::TransactionId as BitcoinTxId;
use ethereum_support::H256 as EthereumTxId;
use ledger_query_service::{
    bitcoin::BitcoinQuery, ethereum::EthereumQuery, Error, LedgerQueryServiceApiClient, QueryId,
};
use std::{fmt, marker::PhantomData, sync::Mutex};
use swap_protocols::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
use tokio::prelude::{future::IntoFuture, Future};

#[derive(Debug)]
pub struct SimpleFakeLedgerQueryService {
    pub bitcoin_results: Vec<BitcoinTxId>,
    pub ethereum_results: Vec<EthereumTxId>,
}

impl LedgerQueryServiceApiClient<Bitcoin, BitcoinQuery> for SimpleFakeLedgerQueryService {
    fn create(
        &self,
        _query: BitcoinQuery,
    ) -> Box<Future<Item = QueryId<Bitcoin>, Error = Error> + Send> {
        Box::new(Ok(QueryId::new("http://localhost/results/1".parse().unwrap())).into_future())
    }

    fn fetch_results(
        &self,
        _query: &QueryId<Bitcoin>,
    ) -> Box<Future<Item = Vec<<Bitcoin as Ledger>::TxId>, Error = Error> + Send> {
        Box::new(Ok(self.bitcoin_results.clone()).into_future())
    }

    fn delete(&self, _query: &QueryId<Bitcoin>) -> Box<Future<Item = (), Error = Error> + Send> {
        unimplemented!()
    }
}

impl LedgerQueryServiceApiClient<Ethereum, EthereumQuery> for SimpleFakeLedgerQueryService {
    fn create(
        &self,
        _query: EthereumQuery,
    ) -> Box<Future<Item = QueryId<Ethereum>, Error = Error> + Send> {
        Box::new(Ok(QueryId::new("http://localhost/results/1".parse().unwrap())).into_future())
    }

    fn fetch_results(
        &self,
        _query: &QueryId<Ethereum>,
    ) -> Box<Future<Item = Vec<<Ethereum as Ledger>::TxId>, Error = Error> + Send> {
        Box::new(Ok(self.ethereum_results.clone()).into_future())
    }

    fn delete(&self, _query: &QueryId<Ethereum>) -> Box<Future<Item = (), Error = Error> + Send> {
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
    fn create(&self, _query: Q) -> Box<Future<Item = QueryId<L>, Error = Error> + Send> {
        Box::new(Ok(QueryId::new("http://localhost/results/1".parse().unwrap())).into_future())
    }

    fn fetch_results(
        &self,
        _query: &QueryId<L>,
    ) -> Box<Future<Item = Vec<L::TxId>, Error = Error> + Send> {
        let mut invocations = self.invocations.lock().unwrap();

        *invocations += 1;

        let transactions = if *invocations >= self.number_of_invocations_before_result {
            self.results.clone()
        } else {
            Vec::new()
        };

        Box::new(Ok(transactions).into_future())
    }

    fn delete(&self, _query: &QueryId<L>) -> Box<Future<Item = (), Error = Error> + Send> {
        unimplemented!()
    }
}

#[derive(DebugStub)]
pub struct LedgerQueryServiceMock<L: Ledger, Q> {
    number_of_invocations: Mutex<u32>,
    #[debug_stub = "next result"]
    results_for_next_invocation:
        Mutex<Option<Box<Future<Item = Vec<L::TxId>, Error = Error> + Send>>>,
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
    pub fn set_next_result(
        &self,
        next_result: Box<Future<Item = Vec<L::TxId>, Error = Error> + Send>,
    ) {
        let mut result = self.results_for_next_invocation.lock().unwrap();

        *result = Some(next_result);
    }

    pub fn number_of_invocations(&self) -> u32 {
        *self.number_of_invocations.lock().unwrap()
    }
}

impl<L: Ledger, Q: fmt::Debug + Send + Sync + 'static> LedgerQueryServiceApiClient<L, Q>
    for LedgerQueryServiceMock<L, Q>
{
    fn create(&self, _query: Q) -> Box<Future<Item = QueryId<L>, Error = Error> + Send> {
        Box::new(Ok(QueryId::new("http://localhost/results/1".parse().unwrap())).into_future())
    }

    fn fetch_results(
        &self,
        _query: &QueryId<L>,
    ) -> Box<Future<Item = Vec<L::TxId>, Error = Error> + Send> {
        let mut invocations = self.number_of_invocations.lock().unwrap();

        let mut results = self.results_for_next_invocation.lock().unwrap();

        let result = results
            .take()
            .unwrap_or_else(|| Box::new(Ok(Vec::new()).into_future()));
        *invocations += 1;

        result
    }

    fn delete(&self, _query: &QueryId<L>) -> Box<Future<Item = (), Error = Error> + Send> {
        unimplemented!()
    }
}
