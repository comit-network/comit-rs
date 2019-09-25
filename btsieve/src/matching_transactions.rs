use futures::Stream;

pub trait MatchingTransactions<Q>: Send + Sync + 'static {
    type Transaction;

    fn matching_transactions(
        &self,
        query: Q,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = ()> + Send>;
}
