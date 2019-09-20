use futures::Stream;

pub trait MatchingTransactions<Q>: Send + Sync + 'static {
    type Error;
    type Transaction;

    fn matching_transactions(
        &self,
        query: Q,
    ) -> Box<dyn Stream<Item = Self::Transaction, Error = Self::Error> + Send>;
}
