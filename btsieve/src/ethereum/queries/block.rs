use crate::{
    ethereum::queries::{to_h256, PayloadKind},
    query_result_repository::QueryResult,
    route_factory::{Error, QueryType, ToHttpPayload},
};
use derivative::Derivative;
use ethereum_support::{
    web3::{transports::Http, types::U256, Web3},
    Block, Transaction,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BlockQuery {
    pub min_timestamp_secs: u64,
}

impl BlockQuery {
    pub fn matches(&self, block: &Block<Transaction>) -> bool {
        let min_timestamp_secs = U256::from(self.min_timestamp_secs);
        min_timestamp_secs <= block.timestamp
    }
}

impl QueryType for BlockQuery {
    fn route() -> &'static str {
        "blocks"
    }
}

#[derive(Deserialize, Derivative, Debug)]
#[derivative(Default)]
#[serde(rename_all = "snake_case")]
pub enum ReturnAs {
    #[derivative(Default)]
    BlockId,
}

impl ToHttpPayload<ReturnAs> for QueryResult {
    type Client = Web3<Http>;
    type Item = PayloadKind;

    fn to_http_payload(
        &self,
        return_as: &ReturnAs,
        _: &Web3<Http>,
    ) -> Result<Vec<Self::Item>, Error> {
        Ok(self
            .0
            .iter()
            .filter_map(to_h256)
            .map(|id| match return_as {
                ReturnAs::BlockId => PayloadKind::Id { id },
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web3::types::{Block, Bytes, Transaction, H160, H2048, H256, U256};
    use spectral::prelude::*;

    fn ethereum_block(timestamp: U256) -> Block<Transaction> {
        Block {
            hash: None,
            parent_hash: H256::from(123),
            uncles_hash: H256::from(123),
            author: H160::from(7),
            state_root: H256::from(123),
            transactions_root: H256::from(123),
            receipts_root: H256::from(123),
            number: None,
            gas_used: U256::from(0),
            gas_limit: U256::from(0),
            extra_data: Bytes::from(vec![]),
            logs_bloom: H2048::from(0),
            timestamp,
            difficulty: U256::from(0),
            total_difficulty: U256::from(0),
            seal_fields: vec![],
            uncles: vec![],
            transactions: vec![],
            size: None,
            mix_hash: None,
            nonce: None,
        }
    }

    #[test]
    fn given_a_block_should_match_smaller_timestamp_query() {
        let block = ethereum_block(U256::from(200));
        let query = BlockQuery {
            min_timestamp_secs: 100u64,
        };

        assert_that(&query.matches(&block)).is_true();
    }

    #[test]
    fn given_a_block_should_non_match_larger_timestamp_query() {
        let block = ethereum_block(U256::from(100));
        let query = BlockQuery {
            min_timestamp_secs: 200u64,
        };

        assert_that(&query.matches(&block)).is_false();
    }
}
