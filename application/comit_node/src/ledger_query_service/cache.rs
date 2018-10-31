use item_cache::ItemCache;
use ledger_query_service::{Error, LedgerQueryServiceApiClient, QueryId};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};
use swap_protocols::ledger::Ledger;
use tokio::prelude::*;

pub struct CachingLedgerQueryServiceApiClientDecorator<L: Ledger, Q, C> {
    query_ids: Mutex<HashMap<Q, ItemCache<QueryId<L>, Error>>>,
    inner: Arc<C>,
}

impl<L: Ledger, Q: Eq + Hash, C> CachingLedgerQueryServiceApiClientDecorator<L, Q, C> {
    pub fn wrap(inner: Arc<C>) -> Self {
        Self {
            query_ids: Mutex::new(HashMap::new()),
            inner,
        }
    }
}

impl<L: Ledger, Q: Eq + Hash + Clone + Send + 'static, C: LedgerQueryServiceApiClient<L, Q>>
    LedgerQueryServiceApiClient<L, Q> for CachingLedgerQueryServiceApiClientDecorator<L, Q, C>
{
    fn create(&self, query: Q) -> Box<Future<Item = QueryId<L>, Error = Error> + Send + 'static> {
        let mut query_ids = self.query_ids.lock().unwrap();

        let query_id = match query_ids.remove(&query) {
            Some(query_id) => query_id,
            None => ItemCache::from_future(self.inner.create(query.clone())),
        };

        let (first, second) = query_id.duplicate();

        query_ids.insert(query, second);

        Box::new(first)
    }

    fn fetch_results(
        &self,
        query: &QueryId<L>,
    ) -> Box<Future<Item = Vec<<L as Ledger>::TxId>, Error = Error> + Send + 'static> {
        self.inner.fetch_results(query)
    }

    fn delete(&self, query: &QueryId<L>) -> Box<Future<Item = (), Error = Error> + Send + 'static> {
        unimplemented!("Cannot delete queries through regular interface, please use delete method on cache decorator.")
    }
}

impl<L: Ledger, Q: Eq + Hash + Clone + Send + 'static, C: LedgerQueryServiceApiClient<L, Q>>
    CachingLedgerQueryServiceApiClientDecorator<L, Q, C>
{
    pub fn delete(
        &self,
        query: Q,
        query_id: &QueryId<L>,
    ) -> Box<Future<Item = (), Error = Error> + Send + 'static> {
        let mut query_ids = self.query_ids.lock().unwrap();

        query_ids.remove(&query);

        self.inner.delete(query_id)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::Arc;
    use swap_protocols::ledger::Bitcoin;
    use tokio::runtime::Runtime;

    #[derive(Default)]
    struct CountInvocations {
        how_many: Mutex<u32>,
    }

    #[derive(PartialOrd, PartialEq, Eq, Hash, Clone)]
    struct SomeQuery {
        criteria: u32,
    }

    impl LedgerQueryServiceApiClient<Bitcoin, SomeQuery> for CountInvocations {
        fn create(
            &self,
            query: SomeQuery,
        ) -> Box<Future<Item = QueryId<Bitcoin>, Error = Error> + Send + 'static> {
            let mut count = self.how_many.lock().unwrap();

            *count += 1;

            let url = format!("http://localhost/some_queries/criteria/{}", query.criteria);

            Box::new(Ok(QueryId::new(url.parse().unwrap())).into_future())
        }

        fn fetch_results(
            &self,
            query: &'_ QueryId<Bitcoin>,
        ) -> Box<Future<Item = Vec<<Bitcoin as Ledger>::TxId>, Error = Error> + Send + 'static>
        {
            unimplemented!()
        }

        fn delete(
            &self,
            query: &'_ QueryId<Bitcoin>,
        ) -> Box<Future<Item = (), Error = Error> + Send + 'static> {
            Box::new(future::ok(()))
        }
    }

    #[test]
    fn given_same_query_returns_original_query_id() {
        let inner = Arc::new(CountInvocations::default());

        let lqs = CachingLedgerQueryServiceApiClientDecorator::wrap(inner.clone());

        let mut runtime = Runtime::new().unwrap();

        let first_location = runtime
            .block_on(lqs.create(SomeQuery { criteria: 10 }))
            .unwrap();

        let second_location = runtime
            .block_on(lqs.create(SomeQuery { criteria: 10 }))
            .unwrap();

        let third_location = runtime
            .block_on(lqs.create(SomeQuery { criteria: 10 }))
            .unwrap();

        let fourth_location = runtime
            .block_on(lqs.create(SomeQuery { criteria: 10 }))
            .unwrap();

        assert_eq!(first_location, second_location);
        assert_eq!(first_location, third_location);
        assert_eq!(first_location, fourth_location);

        let invocations = inner.how_many.lock().unwrap();
        assert_eq!(*invocations, 1);
    }

    #[test]
    fn different_query_results_second_invocation() {
        let inner = Arc::new(CountInvocations::default());

        let lqs = CachingLedgerQueryServiceApiClientDecorator::wrap(inner.clone());

        let mut runtime = Runtime::new().unwrap();

        let first_location = runtime
            .block_on(lqs.create(SomeQuery { criteria: 10 }))
            .unwrap();

        let second_location = runtime
            .block_on(lqs.create(SomeQuery { criteria: 20 }))
            .unwrap();

        assert_ne!(first_location, second_location);

        let invocations = inner.how_many.lock().unwrap();
        assert_eq!(*invocations, 2);
    }

    #[test]
    fn given_second_query_when_first_one_is_deleted_second_one_still_resolves() {
        let inner = Arc::new(CountInvocations::default());

        let lqs = CachingLedgerQueryServiceApiClientDecorator::wrap(inner.clone());

        let mut runtime = Runtime::new().unwrap();

        let first_query = lqs.create(SomeQuery { criteria: 10 });

        let second_query = lqs.create(SomeQuery { criteria: 10 });

        let first_location = runtime.block_on(first_query).unwrap();
        runtime
            .block_on(lqs.delete(SomeQuery { criteria: 10 }, &first_location))
            .unwrap();

        let second_location = runtime.block_on(second_query).unwrap();

        assert_eq!(first_location, second_location);

        let invocations = inner.how_many.lock().unwrap();
        assert_eq!(*invocations, 1);
    }

}
