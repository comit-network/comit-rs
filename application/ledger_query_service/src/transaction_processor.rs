use query_repository::QueryRepository;
use query_result_repository::QueryResultRepository;
use std::sync::Arc;

pub trait TransactionProcessor<T> {
    fn process(&self, transaction: T);
}

pub trait Transaction {
    fn txid(&self) -> String;
}

pub trait Query<T> {
    fn matches(&self, transaction: &T) -> bool;
}

pub struct DefaultTransactionProcessor<Q> {
    queries: Arc<QueryRepository<Q>>,
    results: Arc<QueryResultRepository>,
}

impl<T: Transaction, Q: Query<T> + 'static> TransactionProcessor<T>
    for DefaultTransactionProcessor<Q>
{
    fn process(&self, transaction: T) {
        self.queries
            .all()
            .filter(|(_, query)| query.matches(&transaction))
            .map(|(id, _)| (id, transaction.txid()))
            .for_each(|(query_id, tx_id)| self.results.add_result(query_id, tx_id))
    }
}

impl<Q> DefaultTransactionProcessor<Q> {
    pub fn new(
        query_repository: Arc<QueryRepository<Q>>,
        query_result_repository: Arc<QueryResultRepository>,
    ) -> Self {
        Self {
            queries: query_repository,
            results: query_result_repository,
        }
    }
}
