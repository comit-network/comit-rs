use ethereum_support::{Address, Bytes, Transaction};

#[derive(Clone, Default, Debug, Eq, PartialEq)]
pub struct TransactionQuery {
    pub from_address: Option<Address>,
    pub to_address: Option<Address>,
    pub is_contract_creation: Option<bool>,
    pub transaction_data: Option<Bytes>,
    pub transaction_data_length: Option<usize>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quickcheck::Quickcheck;
    use ethereum_support::web3::types::{Bytes, Transaction};
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
