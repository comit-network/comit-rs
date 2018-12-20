use crate::{
    ledger_query_service::{FetchFullQueryResults, FetchQueryResults, QueryId},
    swap_protocols::ledger::Ledger,
};
use std::sync::Arc;
use tokio::prelude::{stream::iter_ok, *};

pub trait FetchTransactionIdStream<L: Ledger> {
    fn fetch_transaction_id_stream<
        I,
        E: Send + 'static,
        S: Stream<Item = I, Error = E> + Send + 'static,
    >(
        &self,
        ticker: S,
        query_id: QueryId<L>,
    ) -> Box<dyn Stream<Item = L::TxId, Error = S::Error> + Send + 'static>;
}

pub trait FetchTransactionStream<L: Ledger> {
    fn fetch_transaction_stream<
        I,
        E: Send + 'static,
        S: Stream<Item = I, Error = E> + Send + 'static,
    >(
        &self,
        ticker: S,
        query_id: QueryId<L>,
    ) -> Box<dyn Stream<Item = L::Transaction, Error = S::Error> + Send + 'static>;
}

impl<L: Ledger, C> FetchTransactionIdStream<L> for Arc<C>
where
    C: FetchQueryResults<L>,
{
    fn fetch_transaction_id_stream<
        I,
        E: Send + 'static,
        S: Stream<Item = I, Error = E> + Send + 'static,
    >(
        &self,
        ticker: S,
        query_id: QueryId<L>,
    ) -> Box<dyn Stream<Item = <L as Ledger>::TxId, Error = S::Error> + Send + 'static> {
        let mut emitted_transactions = Vec::new();

        let inner_self = Arc::clone(&self);

        Box::new(
            ticker
                .and_then(move |_| {
                    inner_self.fetch_query_results(&query_id).or_else(|e| {
                        warn!("Falling back to empty list of transactions because {:?}", e);
                        Ok(Vec::new())
                    })
                })
                .map(iter_ok)
                .flatten()
                .filter(move |transaction| {
                    let is_new_transaction = !emitted_transactions.contains(transaction);

                    if is_new_transaction {
                        emitted_transactions.push(transaction.clone());
                    }

                    is_new_transaction
                }),
        )
    }
}

impl<L: Ledger> FetchTransactionStream<L> for Arc<dyn FetchFullQueryResults<L>> {
    fn fetch_transaction_stream<
        I,
        E: Send + 'static,
        S: Stream<Item = I, Error = E> + Send + 'static,
    >(
        &self,
        ticker: S,
        query_id: QueryId<L>,
    ) -> Box<dyn Stream<Item = <L as Ledger>::Transaction, Error = S::Error> + Send + 'static> {
        let mut emitted_transactions = Vec::new();

        let inner_self = Arc::clone(&self);

        Box::new(
            ticker
                .and_then(move |_| {
                    inner_self.fetch_full_query_results(&query_id).or_else(|e| {
                        warn!("Falling back to empty list of transactions because {:?}", e);
                        Ok(Vec::new())
                    })
                })
                .map(iter_ok)
                .flatten()
                .filter(move |transaction| {
                    let is_new_transaction = !emitted_transactions.contains(transaction);

                    if is_new_transaction {
                        emitted_transactions.push(transaction.clone());
                    }

                    is_new_transaction
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ledger_query_service::{bitcoin::BitcoinQuery, fake_query_service::LedgerQueryServiceMock},
        swap_protocols::ledger::Bitcoin,
    };
    use bitcoin_support::TransactionId;
    use futures::sync::mpsc;
    use std::time::{Duration, Instant};
    use tokio::{prelude::future::Either, runtime::Runtime, timer::Delay};

    #[test]
    fn should_emit_transactions_as_they_appear_without_waiting_for_the_next_tick() {
        let _ = pretty_env_logger::try_init();

        let mut runtime = Runtime::new().unwrap();

        let (sender, receiver) = mpsc::unbounded();
        let ledger_query_service =
            Arc::new(LedgerQueryServiceMock::<Bitcoin, BitcoinQuery>::default());

        ledger_query_service.set_next_result(Box::new(future::ok(vec![
            TransactionId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            TransactionId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            TransactionId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000003",
            )
            .unwrap(),
        ])));

        let stream = ledger_query_service.fetch_transaction_id_stream(
            receiver,
            QueryId::new("http://localhost/results/1".parse().unwrap()),
        );

        sender.unbounded_send(()).unwrap();
        let (result, stream) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();

        assert_eq!(
            result,
            Some(
                TransactionId::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000001"
                )
                .unwrap()
            )
        );

        let (result, stream) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();
        assert_eq!(
            result,
            Some(
                TransactionId::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000002"
                )
                .unwrap()
            )
        );

        let (result, _) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();
        assert_eq!(
            result,
            Some(
                TransactionId::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000003"
                )
                .unwrap()
            )
        );
        assert_eq!(
            ledger_query_service.number_of_invocations(),
            1,
            "should receive all three results within a single poll"
        );
    }

    #[test]
    fn should_not_emit_same_transaction_twice() {
        let _ = pretty_env_logger::try_init();

        let mut runtime = Runtime::new().unwrap();

        let (sender, receiver) = mpsc::unbounded();
        let ledger_query_service =
            Arc::new(LedgerQueryServiceMock::<Bitcoin, BitcoinQuery>::default());

        ledger_query_service.set_next_result(Box::new(future::ok(vec![TransactionId::from_hex(
            "0000000000000000000000000000000000000000000000000000000000000001",
        )
        .unwrap()])));

        let stream = ledger_query_service.fetch_transaction_id_stream(
            receiver,
            QueryId::new("http://localhost/results/1".parse().unwrap()),
        );

        sender.unbounded_send(()).unwrap();
        let (result, stream) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();

        assert_eq!(
            result,
            Some(
                TransactionId::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000001"
                )
                .unwrap()
            )
        );

        ledger_query_service.set_next_result(Box::new(future::ok(vec![
            TransactionId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            TransactionId::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        ])));

        sender.unbounded_send(()).unwrap();
        let (result, _) = runtime
            .block_on(stream.into_future())
            .map_err(|_| ())
            .unwrap();

        assert_eq!(
            result,
            Some(
                TransactionId::from_hex(
                    "0000000000000000000000000000000000000000000000000000000000000002"
                )
                .unwrap()
            )
        );

        assert_eq!(
            ledger_query_service.number_of_invocations(),
            2,
            "should have polled twice"
        );
    }

    #[test]
    fn given_no_results_should_not_emit_anything() {
        let _ = pretty_env_logger::try_init();

        let mut runtime = Runtime::new().unwrap();
        let (sender, receiver) = mpsc::unbounded();
        let ledger_query_service =
            Arc::new(LedgerQueryServiceMock::<Bitcoin, BitcoinQuery>::default());
        let stream = ledger_query_service.fetch_transaction_id_stream(
            receiver,
            QueryId::new("http://localhost/results/1".parse().unwrap()),
        );

        ledger_query_service.set_next_result(Box::new(future::ok(vec![])));
        sender.unbounded_send(()).unwrap();

        let either = runtime
            .block_on(
                stream
                    .into_future()
                    .select2(Delay::new(Instant::now() + Duration::from_secs(1))),
            )
            .map_err(|_| ())
            .unwrap();

        // A stream of no items will never complete.
        // Thus we `select2` it with a delay that completes after 1 second
        // We have to do this weird assertion because some things are not Debug :(
        // TL;DR: If we don't hit this branch, the Either is Either::B (the timeout) so
        // we are fine.
        if let Either::A(_transaction) = either {
            panic!("should not emit a transaction")
        }
    }
}
