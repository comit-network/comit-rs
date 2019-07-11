use crate::query_result_repository::{QueryResult, QueryResultRepository};
use std::{collections::HashMap, marker::PhantomData, sync::RwLock};

#[derive(Debug, Default)]
pub struct InMemoryQueryResultRepository<Q> {
    storage: RwLock<HashMap<u32, QueryResult>>,
    phantom: PhantomData<Q>,
}

impl<Q: Send + Sync + Clone + 'static> QueryResultRepository<Q>
    for InMemoryQueryResultRepository<Q>
{
    fn get(&self, id: u32) -> Option<QueryResult> {
        let storage = self.storage.read().unwrap();

        storage.get(&id).cloned()
    }

    fn add_result(&self, id: u32, tx_id: String) {
        let mut storage = self.storage.write().unwrap();

        let mut query_result = storage.remove(&id).unwrap_or_default();

        query_result.0.push(tx_id);

        storage.insert(id, query_result);
    }

    fn delete(&self, id: u32) {
        let mut storage = self.storage.write().unwrap();

        storage.remove(&id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;

    #[test]
    fn given_no_entry_can_add_result() {
        let repository = InMemoryQueryResultRepository::<()>::default();

        assert_that(&repository.get(1)).is_none();

        repository.add_result(1, String::from("foobar"));

        assert_that(&repository.get(1))
            .is_some()
            .map(|r| &r.0)
            .contains(String::from("foobar"));
    }

    #[test]
    fn given_existing_entry_adds_result() {
        let repository = InMemoryQueryResultRepository::<()>::default();

        repository.add_result(1, String::from("foobar"));
        repository.add_result(1, String::from("baz"));

        let result = repository.get(1);

        let mut query_results = assert_that(&result).is_some().map(|r| &r.0);

        query_results.contains(String::from("foobar"));
        query_results.contains(String::from("baz"));
    }
}
