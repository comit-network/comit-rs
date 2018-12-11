use crate::{
    item_cache::ItemCache,
    ledger_query_service::{CreateQuery, Error, Query, QueryId},
    swap_protocols::ledger::Ledger,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::prelude::*;

#[derive(Debug)]
pub struct QueryIdCache<L: Ledger, Q: Query> {
    query_ids: Mutex<HashMap<Q, ItemCache<QueryId<L>, Error>>>,
    inner: Arc<dyn CreateQuery<L, Q>>,
}

impl<L: Ledger, Q: Query> QueryIdCache<L, Q> {
    pub fn wrap<C: CreateQuery<L, Q>>(inner: Arc<C>) -> Self {
        Self {
            query_ids: Mutex::new(HashMap::new()),
            inner: inner as Arc<dyn CreateQuery<L, Q>>,
        }
    }
}

impl<L: Ledger, Q: Query> CreateQuery<L, Q> for QueryIdCache<L, Q> {
    fn create_query(
        &self,
        query: Q,
    ) -> Box<dyn Future<Item = QueryId<L>, Error = Error> + Send + 'static> {
        let mut query_ids = self.query_ids.lock().unwrap();

        let query_id = match query_ids.remove(&query) {
            Some(query_id) => {
                trace!("Returning previously stored {:?} for {:?}", query_id, query);
                query_id
            }
            None => ItemCache::from_future(self.inner.create_query(query.clone())),
        };

        let (first, second) = query_id.duplicate();

        query_ids.insert(query, second);

        Box::new(first)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::swap_protocols::ledger::Bitcoin;
    use futures::sync::oneshot::{self, Receiver};
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    #[derive(Default, Debug)]
    struct CountInvocations {
        how_many: Mutex<u32>,
    }

    #[derive(Debug, Clone, Serialize, Eq, Hash, PartialEq)]
    struct SomeQuery {
        criteria: u32,
    }

    impl Query for SomeQuery {}

    impl CreateQuery<Bitcoin, SomeQuery> for CountInvocations {
        fn create_query(
            &self,
            query: SomeQuery,
        ) -> Box<dyn Future<Item = QueryId<Bitcoin>, Error = Error> + Send + 'static> {
            let mut count = self.how_many.lock().unwrap();

            *count += 1;

            let url = format!("http://localhost/some_queries/criteria/{}", query.criteria);

            Box::new(Ok(QueryId::new(url.parse().unwrap())).into_future())
        }
    }

    #[test]
    fn given_same_query_returns_original_query_id() {
        let inner = Arc::new(CountInvocations::default());

        let lqs = QueryIdCache::wrap(inner.clone());

        let mut runtime = Runtime::new().unwrap();

        let first_location = runtime
            .block_on(lqs.create_query(SomeQuery { criteria: 10 }))
            .unwrap();

        let second_location = runtime
            .block_on(lqs.create_query(SomeQuery { criteria: 10 }))
            .unwrap();

        let third_location = runtime
            .block_on(lqs.create_query(SomeQuery { criteria: 10 }))
            .unwrap();

        let fourth_location = runtime
            .block_on(lqs.create_query(SomeQuery { criteria: 10 }))
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

        let lqs = QueryIdCache::wrap(inner.clone());

        let mut runtime = Runtime::new().unwrap();

        let first_location = runtime
            .block_on(lqs.create_query(SomeQuery { criteria: 10 }))
            .unwrap();

        let second_location = runtime
            .block_on(lqs.create_query(SomeQuery { criteria: 20 }))
            .unwrap();

        assert_ne!(first_location, second_location);

        let invocations = inner.how_many.lock().unwrap();
        assert_eq!(*invocations, 2);
    }

    #[derive(Debug)]
    struct Controllable {
        next_response: Mutex<Option<Receiver<QueryId<Bitcoin>>>>,
    }

    impl CreateQuery<Bitcoin, SomeQuery> for Controllable {
        fn create_query(
            &self,
            _: SomeQuery,
        ) -> Box<dyn Future<Item = QueryId<Bitcoin>, Error = Error> + Send + 'static> {
            let mut next_response = self.next_response.lock().unwrap();

            let receiver = next_response.take().unwrap();

            Box::new(
                receiver.map_err(|_| panic!("Controllable doesn't support controlling the error")),
            )
        }
    }

    #[test]
    fn when_future_not_yet_resolved_and_two_times_invoked_both_resolve_to_same_query() {
        let (sender, receiver) = oneshot::channel();

        let controllable = Arc::new(Controllable {
            next_response: Mutex::new(Some(receiver)),
        });

        let lqs = QueryIdCache::wrap(controllable.clone());

        let mut runtime = Runtime::new().unwrap();

        let first_query = lqs.create_query(SomeQuery { criteria: 10 });
        let second_query = lqs.create_query(SomeQuery { criteria: 10 });

        sender
            .send(QueryId::new("http://localhost/foo/bar/".parse().unwrap()))
            .unwrap();

        let first_location = runtime.block_on(first_query).unwrap();

        let second_location = runtime.block_on(second_query).unwrap();

        assert_eq!(first_location, second_location);
    }
}
