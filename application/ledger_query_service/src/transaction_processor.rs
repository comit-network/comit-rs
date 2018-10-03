use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use std::{fmt::Debug, sync::Arc};

pub trait TransactionProcessor<T> {
    fn process(&mut self, transaction: &T);
    fn update_unconfirmed_txs_queue(&mut self);
}

pub trait Transaction: Debug {
    fn transaction_id(&self) -> String;
}

pub trait Query<T>: Debug {
    fn matches(&self, transaction: &T) -> bool;
    fn confirmations_needed(&self, number_of_confirmations: u32) -> u32;
}

pub struct UnconfirmedMatchingTransaction {
    query_id: u32,
    tx_id: String,
    confirmations_still_needed: u32,
}

#[derive(DebugStub)]
pub struct DefaultTransactionProcessor<Q> {
    #[debug_stub = "Queries"]
    queries: Arc<QueryRepository<Q>>,
    #[debug_stub = "Results"]
    results: Arc<QueryResultRepository<Q>>,
    unconfirmed_matching_transactions: Vec<UnconfirmedMatchingTransaction>,
}

impl<T: Transaction, Q: Query<T> + 'static> TransactionProcessor<T>
    for DefaultTransactionProcessor<Q>
{
    fn process(&mut self, transaction: &T) {
        trace!("Processing {:?}", transaction);

        self.queries
            .all()
            .map(|(id, query)| {
                trace!(
                    "Matching query {:#?} against transaction {:#?}",
                    query,
                    transaction
                );
                let tx_matches = query.matches(transaction);

                if tx_matches {
                    let confirmations_needed = query.confirmations_needed(1);
                    let is_confirmed = confirmations_needed <= 0;
                    let tx_id = transaction.transaction_id();

                    if is_confirmed {
                        info!(
                            "Transaction {} matches Query-ID: {:?}",
                            transaction.transaction_id(),
                            id
                        );
                        (true, Some((id, tx_id)), None)
                    } else {
                        (
                            false,
                            None,
                            Some(UnconfirmedMatchingTransaction {
                                query_id: id,
                                tx_id,
                                confirmations_still_needed: confirmations_needed,
                            }),
                        )
                    }
                } else {
                    (false, None, None)
                }
            }).for_each(|(is_result_ready, result, queue_entry)| {
                match (is_result_ready, result, queue_entry) {
                    (true, Some((query_id, tx_id)), None) => {
                        self.results.add_result(query_id, tx_id)
                    }
                    (false, None, Some(unconfirmed_matching_tx)) => {
                        self.unconfirmed_matching_transactions
                            .push(unconfirmed_matching_tx);
                    }
                    _ => (),
                }
            })
    }

    fn update_unconfirmed_txs_queue(&mut self) {
        self.unconfirmed_matching_transactions
            .iter_mut()
            .for_each(|tx| tx.confirmations_still_needed -= 1);

        self.unconfirmed_matching_transactions
            .iter()
            .for_each(|tx| {
                if tx.confirmations_still_needed <= 0 {
                    self.results
                        .add_result(tx.query_id.clone(), tx.tx_id.clone())
                }
            });

        self.unconfirmed_matching_transactions
            .retain(|ref tx| tx.confirmations_still_needed > 0);
    }
}

impl<Q> DefaultTransactionProcessor<Q> {
    pub fn new(
        query_repository: Arc<QueryRepository<Q>>,
        query_result_repository: Arc<QueryResultRepository<Q>>,
    ) -> Self {
        Self {
            queries: query_repository,
            results: query_result_repository,
            unconfirmed_matching_transactions: Vec::new(),
        }
    }
}
