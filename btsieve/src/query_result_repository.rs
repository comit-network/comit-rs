use serde::Serialize;

#[derive(Debug, Serialize, Clone, Default)]
pub struct QueryResult(pub Vec<String>);

pub trait QueryResultRepository<T>: Send + Sync + 'static {
    fn get(&self, id: String) -> Option<QueryResult>;
    fn add_result(&self, id: String, tx_id: String);
    fn delete(&self, id: String);
}
