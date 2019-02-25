use crate::{
    ethereum::queries::{create_transaction_future, to_h256, PayloadKind},
    query_result_repository::QueryResult,
    route_factory::{Error, QueryType, ToHttpPayload},
};
use ethereum_support::{
    web3::{transports::Http, types::H256, Web3},
    Address, Bytes, Transaction,
};
use futures::{
    future::{self, Future},
    stream::{FuturesOrdered, Stream},
};

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct TransactionQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: Option<bool>,
    transaction_data: Option<Bytes>,
    transaction_data_length: Option<usize>,
}

impl TransactionQuery {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        match self {
            Self {
                from_address,
                to_address,
                is_contract_creation,
                transaction_data,
                transaction_data_length,
            } => {
                let mut result = true;

                if let Some(from_address) = from_address {
                    result = result && (transaction.from == *from_address);
                }

                if let Some(to_address) = to_address {
                    result = result && (transaction.to == Some(*to_address));
                }

                if let Some(is_contract_creation) = is_contract_creation {
                    // to_address is None for contract creations
                    result = result && (*is_contract_creation == transaction.to.is_none());
                }

                if let Some(transaction_data) = transaction_data {
                    result = result && (transaction.input == *transaction_data);
                }

                if let Some(transaction_data_length) = transaction_data_length {
                    result = result && (transaction.input.0.len() == *transaction_data_length);
                }
                result
            }
        }
    }
}

impl QueryType for TransactionQuery {
    fn route() -> &'static str {
        "transactions"
    }
}

#[derive(Deserialize, Derivative, Debug)]
#[derivative(Default)]
#[serde(rename_all = "snake_case")]
pub enum ReturnAs {
    #[derivative(Default)]
    TransactionId,
    Transaction,
}

impl ToHttpPayload<TransactionQuery, ReturnAs> for QueryResult {
    type Client = Web3<Http>;
    type Item = PayloadKind;

    fn to_http_payload(
        &self,
        return_as: &ReturnAs,
        client: &Web3<Http>,
    ) -> Result<Vec<Self::Item>, Error> {
        let to_payload = |transaction_id: H256| to_payload(client, transaction_id, return_as);

        self.0
            .iter()
            .filter_map(to_h256)
            .map(to_payload)
            .collect::<FuturesOrdered<_>>()
            .filter_map(|item| item)
            .collect()
            .wait()
    }
}

fn to_payload(
    client: &Web3<Http>,
    transaction_id: H256,
    return_as: &ReturnAs,
) -> Box<dyn Future<Item = Option<PayloadKind>, Error = Error>> {
    match return_as {
        ReturnAs::Transaction => Box::new(create_transaction_future(client, transaction_id).map(
            |maybe_transaction| {
                maybe_transaction.map(|transaction| PayloadKind::Transaction { transaction })
            },
        )),
        ReturnAs::TransactionId => {
            Box::new(future::ok(Some(PayloadKind::Id { id: transaction_id })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web3::types::{Bytes, Transaction, H256, U256};
    use spectral::prelude::*;

    #[test]
    fn given_query_from_address_contract_creation_transaction_matches() {
        let from_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".into();

        let query = TransactionQuery {
            from_address: Some(from_address),
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: None,
            transaction_data_length: None,
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

        let result = query.matches(&transaction);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_query_from_address_doesnt_match() {
        let query = TransactionQuery {
            from_address: Some("a00f2cac7bad9285ecfd59e8860f5b2d8622e099".into()),
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
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

        let result = query.matches(&transaction);
        assert_that(&result).is_false();
    }

    #[test]
    fn given_query_to_address_transaction_matches() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = TransactionQuery {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
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

        let result = query.matches(&transaction);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_query_to_address_transaction_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = TransactionQuery {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
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

        let result = query.matches(&transaction);
        assert_that(&result).is_false();
    }

    #[test]
    fn given_query_to_address_transaction_with_to_none_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = TransactionQuery {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
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

        let result = query.matches(&transaction);
        assert_that(&result).is_false();
    }

    #[test]
    fn given_query_transaction_data_transaction_matches() {
        let query_data = TransactionQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: Some(Bytes::from(vec![1, 2, 3, 4, 5])),
            transaction_data_length: None,
        };

        let query_data_length = TransactionQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: Some(5),
        };

        let refund_query = TransactionQuery {
            from_address: None,
            to_address: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            is_contract_creation: Some(false),
            transaction_data: Some(Bytes::from(vec![])),
            transaction_data_length: None,
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

        let result = query_data.matches(&transaction);
        assert_that(&result).is_true();

        let result = query_data_length.matches(&transaction);
        assert_that(&result).is_true();

        let result = refund_query.matches(&transaction);
        assert_that(&result).is_false();
    }
}
