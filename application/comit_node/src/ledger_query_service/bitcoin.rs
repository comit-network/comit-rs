use bitcoin_support;
use ledger_query_service::Query;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Eq, Hash, PartialEq)]
#[serde(untagged)]
pub enum BitcoinQuery {
    Transaction {
        to_address: Option<bitcoin_support::Address>,
        unlock_script: Option<Vec<Vec<u8>>>,
    },
    Block {
        min_height: Option<u32>,
    },
}

impl Query for BitcoinQuery {}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_support::Address;
    use hex;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn given_a_bitcoin_transaction_query_with_toaddress_it_serializes_ok() {
        let to_address =
            Some(Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap());
        let unlock_script = None;
        let query = BitcoinQuery::Transaction {
            to_address,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"to_address":"bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap","unlock_script":null}"#
        )
    }

    #[test]
    fn given_an_empty_bitcoin_transaction_query_it_serializes_ok() {
        let to_address = None;
        let unlock_script = None;
        let query = BitcoinQuery::Transaction {
            to_address,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"to_address":null,"unlock_script":null}"#)
    }

    #[test]
    fn given_a_bitcoin_block_query_with_min_height_it_serializes_ok() {
        let query = BitcoinQuery::Block {
            min_height: Some(42),
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"min_height":42}"#)
    }

    #[test]
    fn given_a_bitcoin_transaction_query_with_unlock_script_it_serializes_ok() {
        let to_address = None;
        let unlock_script = Some(vec![
            hex::decode("0102030405").unwrap(),
            hex::decode("0504030201").unwrap(),
        ]);
        let query = BitcoinQuery::Transaction {
            to_address,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"to_address":null,"unlock_script":[[1,2,3,4,5],[5,4,3,2,1]]}"#)
    }
}
