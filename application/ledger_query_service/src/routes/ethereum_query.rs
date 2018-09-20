use ethereum_support::{Address, Bytes, Transaction as EthereumTransaction};
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
use transaction_processor::{Query, Transaction};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct EthereumQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: Option<bool>,
    transaction_data: Option<Bytes>,
}

#[post("/queries/ethereum", format = "application/json", data = "<query>")]
pub fn handle_new_ethereum_query<'r>(
    query: Json<EthereumQuery>,
    link_factory: State<LinkFactory>,
    query_repository: State<Arc<QueryRepository<EthereumQuery>>>,
) -> Result<impl Responder<'r>, HttpApiProblem> {
    let query = query.into_inner();

    if let EthereumQuery {
        from_address: None,
        to_address: None,
        is_contract_creation: _, // Not enough by itself
        transaction_data: None,
    } = query
    {
        return Err(HttpApiProblem::with_title_from_status(400)
            .set_detail("Query needs at least one condition"));
    }

    let result = query_repository.save(query);

    match result {
        Ok(id) => Ok(created(
            link_factory.create_link(format!("/queries/ethereum/{}", id)),
        )),
        Err(_) => Err(
            HttpApiProblem::with_title_from_status(500).set_detail("Failed to create new query")
        ),
    }
}

fn created(url: String) -> Created<Option<()>> {
    Created(url, None)
}

impl Query<EthereumTransaction> for EthereumQuery {
    fn matches(&self, transaction: &EthereumTransaction) -> bool {
        let mut result = true;

        if let Some(from_address) = self.from_address {
            result = result && (transaction.from == from_address);
        }

        if let Some(to_address) = self.to_address {
            if let Some(tx_to_address) = transaction.to {
                result = result && (tx_to_address == to_address);
            }
        }

        if let Some(is_contract_creation) = self.is_contract_creation {
            // to_address is None for contract creations
            result = result && (is_contract_creation == transaction.to.is_none());
        }

        if let Some(ref transaction_data) = self.transaction_data {
            result = result && (transaction.input == *transaction_data);
        }

        result
    }
}

impl Transaction for EthereumTransaction {
    fn transaction_id(&self) -> String {
        self.hash.to_string()
    }
}

#[derive(Serialize, Clone, Default)]
pub struct RetrieveEthereumQueryResponse {
    query: EthereumQuery,
    matching_transactions: QueryResult,
}

#[get("/queries/ethereum/<id>")]
pub fn retrieve_ethereum_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumQuery>>>,
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

#[delete("/queries/ethereum/<id>")]
pub fn delete_ethereum_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<EthereumQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<EthereumQuery>>>,
) -> impl Responder<'static> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    NoContent
}

#[cfg(test)]
mod tests {
    use super::*;
    use spectral::prelude::*;
    use web3::types::{Bytes, H256, Transaction, U256};

    #[test]
    fn given_query_from_address_contract_creation_transaction_matches() {
        let from_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumQuery {
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

        assert_that(&query.matches(&transaction)).is_true();
    }

    #[test]
    fn given_query_from_address_doesnt_match() {
        let query = EthereumQuery {
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

        assert_that(&query.matches(&transaction)).is_false();
    }

    #[test]
    fn given_query_to_address_transaction_matches() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumQuery {
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

        assert_that(&query.matches(&transaction)).is_true();
    }

    #[test]
    fn given_query_to_address_transaction_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = EthereumQuery {
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

        assert_that(&query.matches(&transaction)).is_false();
    }

    #[test]
    fn given_query_transaction_data_transaction_matches() {
        let query = EthereumQuery {
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

        assert_that(&query.matches(&transaction)).is_true();
    }

    #[test]
    fn given_no_conditions_in_query_transaction_matches() {
        let query = EthereumQuery {
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

        assert_that(&query.matches(&transaction)).is_true();
    }
}
