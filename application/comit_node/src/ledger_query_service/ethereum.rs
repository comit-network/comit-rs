use ethereum_support::web3::types::{Address, Bytes};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum EthereumQuery {
    Transaction {
        from_address: Option<Address>,
        to_address: Option<Address>,
        is_contract_creation: Option<bool>,
        transaction_data: Option<Bytes>,
    },
    Block {
        min_timestamp_secs: Option<u32>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_support::Address;
    use serde_json;
    use std::str::FromStr;

    #[test]
    fn given_a_ethereum_transaction_query_with_toaddress_it_serializes_ok() {
        let to_address =
            Some(Address::from_str("8457037fcd80a8650c4692d7fcfc1d0a96b92867").unwrap());
        let query = EthereumQuery::Transaction {
            from_address: None,
            to_address,
            is_contract_creation: None,
            transaction_data: None,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"from_address":null,"to_address":"0x8457037fcd80a8650c4692d7fcfc1d0a96b92867","is_contract_creation":null,"transaction_data":null}"#
        )
    }

    #[test]
    fn given_an_empty_ethereum_transaction_query_it_serializes_ok() {
        let to_address = None;
        let query = EthereumQuery::Transaction {
            from_address: None,
            to_address,
            is_contract_creation: None,
            transaction_data: None,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"from_address":null,"to_address":null,"is_contract_creation":null,"transaction_data":null}"#
        )
    }

    #[test]
    fn given_a_ethereum_block_query_with_min_timestamp_it_serializes_ok() {
        let query = EthereumQuery::Block {
            min_timestamp_secs: Some(10),
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"min_timestamp_secs":10}"#)
    }
}
