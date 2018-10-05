use failure::Error;
use futures_ext::{PollUntilReady, StreamTemplate};
use ledger_query_service::{LedgerQueryServiceApiClient, QueryId};
use std::{sync::Arc, time::Duration};
use swap_protocols::ledger::Ledger;
use tokio::prelude::*;

#[derive(Clone, Debug)]
pub struct LedgerServices<L: Ledger, Q> {
    api_client: Arc<LedgerQueryServiceApiClient<L, Q>>,
    poll_interval: Duration,
}

impl<L: Ledger, Q> LedgerServices<L, Q> {
    pub fn new(
        api_client: Arc<LedgerQueryServiceApiClient<L, Q>>,
        poll_interval: Duration,
    ) -> LedgerServices<L, Q> {
        LedgerServices {
            api_client,
            poll_interval,
        }
    }
}

#[derive(Debug)]
pub struct TransactionIdStream<F, L: Ledger> {
    inner: F,
    transactions: Vec<L::TxId>,
    next_index: usize,
}

impl<F: Future<Item = Vec<L::TxId>, Error = Error>, L: Ledger> Stream
    for TransactionIdStream<F, L>
{
    type Item = L::TxId;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<<Self as Stream>::Item>>, <Self as Stream>::Error> {
        trace!(
            "Polling Stream. Got {} transactions in total, {} already emitted.",
            self.transactions.len(),
            self.next_index
        );

        if let Some(transaction) = self.transactions.get(self.next_index) {
            self.next_index += 1;

            trace!("Emitting transaction {:?}.", transaction);

            return Ok(Async::Ready(Some(transaction.clone())));
        }

        let transactions = try_ready!(self.inner.poll());

        trace!(
            "Got new transactions ({}) from inner future: {:?}. Currently have {} transactions.",
            transactions.len(),
            transactions,
            self.transactions.len()
        );

        self.transactions = transactions;

        if self.transactions.len() > self.next_index {
            self.poll()
        } else {
            Ok(Async::NotReady)
        }
    }
}

#[derive(Debug)]
pub struct FetchQueryResultsFuture<L: Ledger, Q> {
    query_id: QueryId<L>,
    api_client: Arc<LedgerQueryServiceApiClient<L, Q>>,
}

impl<L: Ledger, Q: 'static> Future for FetchQueryResultsFuture<L, Q> {
    type Item = Vec<L::TxId>;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<<Self as Future>::Item>, <Self as Future>::Error> {
        self.api_client
            .fetch_results(&self.query_id)
            .into_future()
            .poll()
    }
}

impl<L: Ledger, Q: 'static> StreamTemplate<LedgerServices<L, Q>> for QueryId<L> {
    type Stream = TransactionIdStream<PollUntilReady<FetchQueryResultsFuture<L, Q>>, L>;

    fn into_stream(
        self,
        dependencies: LedgerServices<L, Q>,
    ) -> TransactionIdStream<PollUntilReady<FetchQueryResultsFuture<L, Q>>, L> {
        TransactionIdStream {
            inner: {
                PollUntilReady::new(
                    FetchQueryResultsFuture {
                        query_id: self,
                        api_client: dependencies.api_client,
                    },
                    dependencies.poll_interval,
                )
            },
            transactions: Vec::new(),
            next_index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_ext::FutureFactory;
    use ledger_query_service::{
        fake_query_service::InvocationCountFakeLedgerQueryService, BitcoinQuery,
    };
    use spectral::prelude::*;
    use std::sync::Mutex;
    use swap_protocols::ledger::bitcoin::Bitcoin;
    use tokio::runtime::Runtime;

    #[test]
    fn given_future_resolves_to_transaction_eventually() {
        let ledger_query_service = Arc::new(InvocationCountFakeLedgerQueryService::<Bitcoin> {
            number_of_invocations_before_result: 5,
            invocations: Mutex::new(0),
            results: vec![
                "7e7c52b1f46e7ea2511e885d8c0e5df9297f65b6fff6907ceb1377d0582e45f4"
                    .parse()
                    .unwrap(),
            ],
        });

        let future_factory = FutureFactory::new(LedgerServices::<Bitcoin, BitcoinQuery>::new(
            ledger_query_service.clone(),
            Duration::from_millis(100),
        ));

        let stream = future_factory.create_stream_from_template(QueryId::new(
            "http://localhost/results/1".parse().unwrap(),
        ));

        let mut _runtime = Runtime::new().unwrap();

        let result = _runtime.block_on(stream.into_future());
        let result = result.map(|(item, _stream)| item).map_err(|(e1, _e2)| e1);

        let invocations = ledger_query_service.invocations.lock().unwrap();

        assert_that(&*invocations).is_equal_to(5);
        assert_that(&result).is_ok().is_some();
    }
}
