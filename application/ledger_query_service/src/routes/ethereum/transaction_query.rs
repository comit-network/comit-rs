use block_processor::{Block, IsEqualTo, Query, QueryMatchResult, Transaction};
use ethereum_support::{
    Address, Block as EthereumBlock, Bytes, Transaction as EthereumTransaction,
};
use http_api_problem::HttpApiProblem;
use link_factory::LinkFactory;
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
pub struct EthereumTransactionQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: Option<bool>,
    transaction_data: Option<Bytes>,
}

#[post(
    "/queries/ethereum/transactions",
    format = "application/json",
    data = "<query>"
)]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn handle_new_query<'r>(
    query: Json<EthereumTransactionQuery>,
    link_factory: State<LinkFactory>,
    query_repository: State<Arc<QueryRepository<EthereumTransactionQuery>>>,
) -> Result<impl Responder<'r>, HttpApiProblem> {
    let query = query.into_inner();

    if let EthereumTransactionQuery {
        from_address: None,
        to_address: None,
        transaction_data: None,
        ..
    } = query
    {
        return Err(HttpApiProblem::with_title_from_status(400)
            .set_detail("Query needs at least one condition"));
    }

    let result = query_repository.save(query);

    match result {
        Ok(id) => Ok(created(
            link_factory.create_link(format!("/queries/ethereum/transactions/{}", id)),
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

impl Query<EthereumTransaction> for EthereumTransactionQuery {
    fn matches(&self, transaction: &EthereumTransaction) -> QueryMatchResult {
        let EthereumTransactionQuery {
            from_address,
            to_address,
            is_contract_creation,
            transaction_data,
        } = self;

        let matches_from_address = from_address.is_equal_to(|| transaction.from);
        let matches_to_address = to_address.is_equal_to(|| &transaction.to);
        let is_contract_creation = is_contract_creation.is_equal_to(|| transaction.to.is_none());
        let matches_transaction_data = transaction_data.is_equal_to(|| &transaction.input);

        matches_from_address
            .or(matches_to_address)
            .or(is_contract_creation)
            .or(matches_transaction_data)
    }
}

impl Transaction for EthereumTransaction {
    fn transaction_id(&self) -> String {
        format!("{:?}", self.hash)
    }
}

impl Block for EthereumBlock<EthereumTransaction> {
    type Transaction = EthereumTransaction;
    fn blockhash(&self) -> String {
        format!("{:x}", self.hash.unwrap())
    }
    fn prev_blockhash(&self) -> String {
        format!("{:x}", self.parent_hash)
    }
    fn transactions(&self) -> &[Self::Transaction] {
        self.transactions.as_slice()
    }
}

#[derive(Serialize, Clone, Default, Debug)]
pub struct RetrieveEthereumQueryResponse {
    query: EthereumTransactionQuery,
    matching_transactions: QueryResult,
}

#[get("/queries/ethereum/transactions/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn retrieve_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumTransactionQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumTransactionQuery>>>,
) -> Result<Json<RetrieveEthereumQueryResponse>, HttpApiProblem> {
    let query = query_repository.get(id).ok_or_else(|| {
        HttpApiProblem::with_title_from_status(404).set_detail("The requested query does not exist")
    })?;

    let result = query_result_repository.get(id).unwrap_or_default();

    Ok(Json(RetrieveEthereumQueryResponse {
        query,
        matching_transactions: result,
    }))
}

#[delete("/queries/ethereum/transactions/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn delete_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumTransactionQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumTransactionQuery>>>,
) -> impl Responder<'static> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    NoContent
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use web3::types::{Bytes, Transaction, H256, U256};

    #[test]
    fn given_query_from_address_contract_creation_transaction_matches() {
        let from_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumTransactionQuery {
            from_address: Some(from_address),
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: None,
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: from_address,
            to: None, // None = contract creation
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };

        assert_that(&query.matches(&transaction))
            .is_equal_to(QueryMatchResult::yes_with_confirmations(0));
    }

    #[test]
    fn given_query_from_address_doesnt_match() {
        let query = EthereumTransactionQuery {
            from_address: Some("a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap()),
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "a00f2cac7bad9285ecfd59e8860f5b2dffffffff".parse().unwrap(),
            to: None, // None = contract creation
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };

        assert_that(&query.matches(&transaction)).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_query_to_address_transaction_matches() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumTransactionQuery {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some(to_address),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };

        assert_that(&query.matches(&transaction))
            .is_equal_to(QueryMatchResult::yes_with_confirmations(0));
    }

    #[test]
    fn given_query_to_address_transaction_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumTransactionQuery {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap()),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };

        assert_that(&query.matches(&transaction)).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_query_to_address_transaction_with_to_none_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumTransactionQuery {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: None,
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        };

        assert_that(&query.matches(&transaction)).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_query_transaction_data_transaction_matches() {
        let query = EthereumTransactionQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: Some(Bytes::from(vec![1, 2, 3, 4, 5])),
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![1, 2, 3, 4, 5]),
        };

        assert_that(&query.matches(&transaction))
            .is_equal_to(QueryMatchResult::yes_with_confirmations(0));
    }

    #[test]
    fn given_no_conditions_in_query_transaction_fails() {
        let query = EthereumTransactionQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
        };

        let transaction = Transaction {
            hash: H256::from(123),
            nonce: U256::from(1),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![1, 2, 3, 4, 5]),
        };

        assert_that(&query.matches(&transaction)).is_equal_to(QueryMatchResult::no());
    }

}
