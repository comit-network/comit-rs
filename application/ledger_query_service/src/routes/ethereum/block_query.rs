use block_processor::Query;
use ethereum_support::web3::types::{
    Block as EthereumBlock, Transaction as EthereumTransaction, U256,
};
use http_api_problem::HttpApiProblem;
use link_factory::LinkFactory;
use query_match_result::{Matches, QueryMatchResult};
use query_repository::QueryRepository;
use query_result_repository::{QueryResult, QueryResultRepository};
use rocket::{
    response::{
        status::{Created, NoContent},
        Responder,
    },
    State,
};
use rocket_contrib::Json;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct EthereumBlockQuery {
    pub min_timestamp_secs: Option<u64>,
}

#[post(
    "/queries/ethereum/blocks",
    format = "application/json",
    data = "<query>"
)]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn handle_new_query<'r>(
    query: Json<EthereumBlockQuery>,
    link_factory: State<LinkFactory>,
    query_repository: State<Arc<QueryRepository<EthereumBlockQuery>>>,
) -> Result<impl Responder<'r>, HttpApiProblem> {
    let query = query.into_inner();

    if let EthereumBlockQuery {
        min_timestamp_secs: None,
        ..
    } = query
    {
        return Err(HttpApiProblem::with_title_from_status(400)
            .set_detail("Query needs at least one condition"));
    }

    let result = query_repository.save(query);

    match result {
        Ok(id) => Ok(created(
            link_factory.create_link(format!("/queries/ethereum/blocks/{}", id)),
        )),
        Err(_) => {
            Err(HttpApiProblem::with_title_from_status(500)
                .set_detail("Failed to create new query"))
        }
    }
}

fn created(url: String) -> Created<Option<()>> {
    Created(url, None)
}

impl Query<EthereumBlock<EthereumTransaction>> for EthereumBlockQuery {
    fn matches(&self, block: &EthereumBlock<EthereumTransaction>) -> QueryMatchResult {
        self.min_timestamp_secs
            .matches(|minimum_timestamp| U256::from(*minimum_timestamp) <= block.timestamp)
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct RetrieveEthereumBlockQueryResponse {
    query: EthereumBlockQuery,
    matching_blocks: QueryResult,
}

#[get("/queries/ethereum/blocks/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn retrieve_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumBlockQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumBlockQuery>>>,
) -> Result<Json<RetrieveEthereumBlockQueryResponse>, HttpApiProblem> {
    let query = query_repository.get(id).ok_or_else(|| {
        HttpApiProblem::with_title_from_status(404).set_detail("The requested query does not exist")
    })?;

    let result = query_result_repository.get(id).unwrap_or_default();

    Ok(Json(RetrieveEthereumBlockQueryResponse {
        query,
        matching_blocks: result,
    }))
}

#[delete("/queries/ethereum/blocks/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn delete_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumBlockQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumBlockQuery>>>,
) -> impl Responder<'static> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    NoContent
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_support::{Block, Bytes, H160, H2048, H256};
    use spectral::prelude::*;

    #[test]
    fn given_query_min_timestamp_then_older_block_does_not_match() {
        let block = Block {
            hash: Some(H256::zero()),
            parent_hash: H256::zero(),
            uncles_hash: H256::zero(),
            author: H160::zero(),
            state_root: H256::zero(),
            transactions_root: H256::zero(),
            receipts_root: H256::zero(),
            number: None,
            gas_used: U256::from(0),
            gas_limit: U256::from(0),
            extra_data: Bytes::from(vec![]),
            logs_bloom: H2048::zero(),
            timestamp: U256::from(9000),
            difficulty: U256::from(0),
            total_difficulty: U256::from(0),
            seal_fields: vec![Bytes::from(vec![])],
            uncles: vec![],
            transactions: vec![],
            size: None,
        };

        let query = EthereumBlockQuery {
            min_timestamp_secs: Some(10_000),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_query_min_timestamp_then_exact_block_matches() {
        let block = Block {
            hash: Some(H256::zero()),
            parent_hash: H256::zero(),
            uncles_hash: H256::zero(),
            author: H160::zero(),
            state_root: H256::zero(),
            transactions_root: H256::zero(),
            receipts_root: H256::zero(),
            number: None,
            gas_used: U256::from(0),
            gas_limit: U256::from(0),
            extra_data: Bytes::from(vec![]),
            logs_bloom: H2048::zero(),
            timestamp: U256::from(10_000),
            difficulty: U256::from(0),
            total_difficulty: U256::from(0),
            seal_fields: vec![Bytes::from(vec![])],
            uncles: vec![],
            transactions: vec![],
            size: None,
        };

        let query = EthereumBlockQuery {
            min_timestamp_secs: Some(10_000),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::yes());
    }

    #[test]
    fn given_query_min_timestamp_then_younger_block_matches() {
        let block = Block {
            hash: Some(H256::zero()),
            parent_hash: H256::zero(),
            uncles_hash: H256::zero(),
            author: H160::zero(),
            state_root: H256::zero(),
            transactions_root: H256::zero(),
            receipts_root: H256::zero(),
            number: None,
            gas_used: U256::from(0),
            gas_limit: U256::from(0),
            extra_data: Bytes::from(vec![]),
            logs_bloom: H2048::zero(),
            timestamp: U256::from(11_000),
            difficulty: U256::from(0),
            total_difficulty: U256::from(0),
            seal_fields: vec![Bytes::from(vec![])],
            uncles: vec![],
            transactions: vec![],
            size: None,
        };

        let query = EthereumBlockQuery {
            min_timestamp_secs: Some(10_000),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::yes());
    }

}
