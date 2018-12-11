use crate::ledger_query_service::Query;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Eq, Hash, PartialEq)]
#[serde(untagged)]
pub enum BitcoinQuery {
    Transaction {
        to_address: Option<bitcoin_support::Address>,
        from_outpoint: Option<bitcoin_support::OutPoint>,
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
    use bitcoin_support::{Address, OutPoint, Sha256dHash};
    use hex;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn given_a_bitcoin_transaction_query_with_toaddress_it_serializes_ok() {
        let to_address =
            Some(Address::from_str("bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap").unwrap());
        let from_outpoint = None;
        let unlock_script = None;
        let query = BitcoinQuery::Transaction {
            to_address,
            from_outpoint,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"to_address":"bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap","from_outpoint":null,"unlock_script":null}"#
        )
    }

    #[test]
    fn given_an_empty_bitcoin_transaction_query_it_serializes_ok() {
        let to_address = None;
        let from_outpoint = None;
        let unlock_script = None;
        let query = BitcoinQuery::Transaction {
            to_address,
            from_outpoint,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"to_address":null,"from_outpoint":null,"unlock_script":null}"#
        )
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
    fn given_a_bitcoin_transaction_query_with_from_outpoint_it_serializes_ok() {
        let to_address = None;
        let unlock_script = None;
        let from_outpoint = Some(OutPoint {
            txid: Sha256dHash::from_hex(
                "02b082113e35d5386285094c2829e7e2963fa0b5369fb7f4b79c4c90877dcd3d",
            )
            .unwrap(),
            vout: 0u32,
        });

        let query = BitcoinQuery::Transaction {
            to_address,
            from_outpoint,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"to_address":null,"from_outpoint":{"txid":"02b082113e35d5386285094c2829e7e2963fa0b5369fb7f4b79c4c90877dcd3d","vout":0},"unlock_script":null}"#
        )
    }

    #[test]
    fn given_a_bitcoin_transaction_query_with_unlock_script_it_serializes_ok() {
        let to_address = None;
        let from_outpoint = None;
        let unlock_script = Some(vec![
            hex::decode("0102030405").unwrap(),
            hex::decode("0504030201").unwrap(),
        ]);
        let query = BitcoinQuery::Transaction {
            to_address,
            from_outpoint,
            unlock_script,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"to_address":null,"from_outpoint":null,"unlock_script":[[1,2,3,4,5],[5,4,3,2,1]]}"#
        )
    }
}
