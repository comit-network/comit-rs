use crate::{
    ethereum::queries::{create_transaction_future, to_h256, PayloadKind},
    query_result_repository::QueryResult,
    route_factory::{Error, QueryType, ToHttpPayload},
};
use derivative::Derivative;
use ethereum_support::{
    web3::{transports::Http, types::H256, Web3},
    Address, Bytes, Transaction,
};
use futures::{
    future::{self, Future},
    stream::{FuturesOrdered, Stream},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default, Debug, Eq, PartialEq)]
pub struct TransactionQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: bool,
    transaction_data: Option<Bytes>,
    transaction_data_length: Option<usize>,
}

impl TransactionQuery {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        if let Some(from_address) = &self.from_address {
            if transaction.from != *from_address {
                return false;
            }
        }

        if let Some(to_address) = &self.to_address {
            if transaction.to != Some(*to_address) {
                return false;
            }
        }

        if self.is_contract_creation && transaction.to.is_some() {
            return false;
        }

        if let Some(transaction_data) = &self.transaction_data {
            if transaction.input != *transaction_data {
                return false;
            }
        }

        if let Some(transaction_data_length) = &self.transaction_data_length {
            if transaction.input.0.len() != *transaction_data_length {
                return false;
            }
        }

        true
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

impl ToHttpPayload<ReturnAs> for QueryResult {
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
            .collect()
            .wait()
    }
}

fn to_payload(
    client: &Web3<Http>,
    transaction_id: H256,
    return_as: &ReturnAs,
) -> Box<dyn Future<Item = PayloadKind, Error = Error>> {
    match return_as {
        ReturnAs::Transaction => Box::new(
            create_transaction_future(client, transaction_id)
                .map(|transaction| PayloadKind::Transaction { transaction }),
        ),
        ReturnAs::TransactionId => Box::new(future::ok(PayloadKind::Id { id: transaction_id })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ethereum::queries::quickcheck::Quickcheck,
        web3::types::{Bytes, Transaction},
    };
    use spectral::prelude::*;

    #[test]
    fn given_query_from_arbitrary_address_contract_creation_transaction_matches() {
        fn prop(from_address: Quickcheck<Address>, transaction: Quickcheck<Transaction>) -> bool {
            let from_address = from_address.0;

            let query = TransactionQuery {
                from_address: Some(from_address),
                to_address: None,
                is_contract_creation: Some(true),
                transaction_data: None,
                transaction_data_length: None,
            };

            let mut transaction = transaction.0;

            transaction.from = from_address;
            transaction.to = None;

            query.matches(&transaction)
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<Address>, Quickcheck<Transaction>) -> bool)
    }

    #[test]
    fn given_query_from_address_doesnt_match() {
        let from_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let query = TransactionQuery {
            from_address: Some(from_address),
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
        };

        let transaction = Transaction {
            from: "a00f2cac7bad9285ecfd59e8860f5b2dffffffff".parse().unwrap(),
            ..Transaction::default()
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
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some(to_address),
            ..Transaction::default()
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
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap()),
            ..Transaction::default()
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
            to: None,
            ..Transaction::default()
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
            to: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            input: Bytes::from(vec![1, 2, 3, 4, 5]),
            ..Transaction::default()
        };

        let result = query_data.matches(&transaction);
        assert_that(&result).is_true();

        let result = query_data_length.matches(&transaction);
        assert_that(&result).is_true();

        let result = refund_query.matches(&transaction);
        assert_that(&result).is_false();
    }
}
