#[allow(dead_code)]
pub struct Query {
    pub(crate) location: String,
}

#[derive(Serialize)]
pub struct BitcoinQuery {
    pub to_address: Option<String>,
}

pub trait LedgerQueryService: Send + Sync {
    fn create_bitcoin_query(&self, query: BitcoinQuery) -> Result<Query, ()>;
    fn fetch_query_results(&self, query: &Query) -> Result<Vec<String>, ()>;
    fn delete_query(&self, query: &Query);
}
