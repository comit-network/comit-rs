pub mod transaction_query;
pub mod block_query {
    use block_processor::{Query, QueryMatchResult};
    use ethereum_support::web3::types::{Block, Transaction};

    #[derive(Clone, Debug)]
    pub struct EthereumBlockQuery {}

    impl Query<Block<Transaction>> for EthereumBlockQuery {
        fn matches(&self, object: &'_ Block<Transaction>) -> QueryMatchResult {
            unimplemented!()
        }
    }
}
