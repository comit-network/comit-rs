#[derive(Debug, Serialize, Clone, Default)]
pub struct QueryResult(pub Vec<String>);

pub trait QueryResultRepository<T>: Send + Sync + 'static {
    fn get(&self, id: u32) -> Option<QueryResult>;
    fn add_result(&self, id: u32, tx_id: String);
    fn delete(&self, id: u32);
}
