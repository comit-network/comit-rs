use bitcoin_rpc_client::{BitcoinRpcApi, TransactionId};
use bitcoin_support::{Address, Transaction};
use future_template::FutureTemplate;
use futures::{Async, Future};
use ganp::ledger::{bitcoin::Bitcoin, Ledger};
use ledger_query_service::{BitcoinQuery, LedgerQueryService, Query};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_timer::Delay;

#[derive(Clone)]
pub struct LedgerServices {
    bitcoin_node: Arc<BitcoinRpcApi>,
    ledger_query_service: Arc<LedgerQueryService>,
}

pub struct PaymentToAddress<L: Ledger> {
    to_address: L::Address,
}

pub struct PaymentToBitcoinAddressFuture {
    to_address: Address,
    bitcoin_node: Arc<BitcoinRpcApi>,
    ledger_query_service: Arc<LedgerQueryService>,
    query: Option<Query>,
    next_try: Delay,
}

impl Future for PaymentToBitcoinAddressFuture {
    type Item = Transaction;
    type Error = ();

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        if let Ok(Async::NotReady) = self.next_try.poll() {
            return Ok(Async::NotReady);
        }

        let result = match self.query {
            Some(ref query) => match self.ledger_query_service.fetch_query_results(query) {
                Ok(ref results) if results.len() > 0 => match self
                    .bitcoin_node
                    .get_raw_transaction_serialized(&results.get(0).unwrap().parse().unwrap())
                {
                    Ok(Ok(transaction)) => Ok(Async::Ready(transaction.into())),
                    _ => Ok(Async::NotReady),
                },
                _ => Ok(Async::NotReady),
            },
            None => match self
                .ledger_query_service
                .create_bitcoin_query(BitcoinQuery {
                    to_address: Some(self.to_address.to_string()),
                }) {
                Ok(query) => {
                    self.query = Some(query);
                    Ok(Async::NotReady)
                }
                Err(e) => {
                    warn!("Failed to create query: {:?}", e);
                    Ok(Async::NotReady)
                }
            },
        };

        match result {
            Ok(Async::NotReady) => {
                self.next_try = Delay::new(Instant::now() + Duration::from_millis(500));
                self.poll()
            }
            _ => result,
        }
    }
}

impl FutureTemplate<LedgerServices> for PaymentToAddress<Bitcoin> {
    type Future = PaymentToBitcoinAddressFuture;

    fn into_future(self, dependencies: LedgerServices) -> PaymentToBitcoinAddressFuture {
        PaymentToBitcoinAddressFuture {
            to_address: self.to_address,
            bitcoin_node: dependencies.bitcoin_node,
            ledger_query_service: dependencies.ledger_query_service,
            query: None,
            next_try: Delay::new(Instant::now() + Duration::from_millis(500)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_rpc_client::{RpcError, SerializedRawTransaction};
    use env_logger;
    use future_template::FutureFactory;
    use reqwest;
    use spectral::prelude::*;
    use std::sync::Mutex;
    use tokio::runtime::Runtime;

    struct FakeBitcoinRpcApi;

    impl BitcoinRpcApi for FakeBitcoinRpcApi {
        fn get_raw_transaction_serialized(
            &self,
            _tx: &TransactionId,
        ) -> Result<Result<SerializedRawTransaction, RpcError>, reqwest::Error> {
            Ok(Ok(SerializedRawTransaction::from("0200000000010144af9381cd3cb3d14d549b27c8d8a4c87d1d58e501df656342363886277f62e10000000000feffffff02aba9ac0300000000160014908abcc05defb6ba5630268b395b1fab19ad50d760566c0000000000220020c39353c0df01296ab055e83b701715b765636cf91c795deb7573e4b055ada53302473044022010d3b0f0e48977b5c7af7f6a0839a8ed24cd760c4e95668ed7b3275fca727360022007a27825d82a1e69bff2e8cbf195aa4280c214f1cf7650afb6fa2eb49a9765040121036bc4598b0de6ac9c560f1322ce86a0bf27e934837ac86196337db06002c3a352f83a1400")))
        }
    }

    struct FakeLedgerQueryService {
        number_of_invocations_before_result: u32,
        invocations: Mutex<u32>,
        results: Vec<String>,
    }

    impl LedgerQueryService for FakeLedgerQueryService {
        fn create_bitcoin_query(&self, _query: BitcoinQuery) -> Result<Query, ()> {
            Ok(Query {
                location: String::new(),
            })
        }

        fn fetch_query_results(&self, _query: &Query) -> Result<Vec<String>, ()> {
            let mut invocations = self.invocations.lock().unwrap();

            *invocations += 1;

            if *invocations >= self.number_of_invocations_before_result {
                Ok(self.results.clone())
            } else {
                Ok(Vec::new())
            }
        }

        fn delete_query(&self, _query: &Query) {
            unimplemented!()
        }
    }

    #[test]
    fn given_future_resolves_to_transaction_eventually() {
        let _ = env_logger::try_init();

        let bitcoin_rpc_api = FakeBitcoinRpcApi;

        let ledger_query_service = Arc::new(FakeLedgerQueryService {
            number_of_invocations_before_result: 5,
            invocations: Mutex::new(0),
            results: vec![String::from(
                "7e7c52b1f46e7ea2511e885d8c0e5df9297f65b6fff6907ceb1377d0582e45f4",
            )],
        });

        let future_factory = FutureFactory::new(LedgerServices {
            bitcoin_node: Arc::new(bitcoin_rpc_api),
            ledger_query_service: ledger_query_service.clone(),
        });

        let payment_to_address = PaymentToAddress {
            to_address: "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap"
                .parse()
                .unwrap(),
        };

        let future = future_factory.create_future_from_template(payment_to_address);

        let mut runtime = Runtime::new().unwrap();

        let result = runtime.block_on(future);

        let invocations = ledger_query_service.invocations.lock().unwrap();

        assert_that(&*invocations).is_equal_to(5);
        assert_that(&result).is_ok();
    }
}
