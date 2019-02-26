use crate::{
    btsieve::{Error, Query, QueryId},
    swap_protocols::ledger::Ethereum,
};
use ethereum_support::{
    web3::types::{Address, Bytes, Transaction, H256},
    TransactionAndReceipt,
};
use futures::Future;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Eq, Hash, PartialEq)]
#[serde(untagged)]
pub enum EthereumQuery {
    Transaction {
        from_address: Option<Address>,
        to_address: Option<Address>,
        is_contract_creation: Option<bool>,
        transaction_data: Option<Bytes>,
        transaction_data_length: Option<usize>,
    },
    Block {
        min_timestamp_secs: u32,
    },
    Event {
        event_matchers: Vec<EventMatcher>,
    },
}

impl EthereumQuery {
    pub fn contract_deployment(contract_data: Bytes) -> Self {
        EthereumQuery::Transaction {
            from_address: None,
            to_address: None,
            is_contract_creation: Some(true),
            transaction_data: Some(contract_data),
            transaction_data_length: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct EventMatcher {
    pub address: Option<Address>,
    pub data: Option<Bytes>,
    pub topics: Vec<Option<Topic>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct Topic(pub H256);

impl Query for EthereumQuery {}

pub trait QueryEthereum {
    fn create(
        &self,
        query: EthereumQuery,
    ) -> Box<dyn Future<Item = QueryId<Ethereum>, Error = Error> + Send>;

    fn delete(&self, query: &QueryId<Ethereum>)
        -> Box<dyn Future<Item = (), Error = Error> + Send>;
    fn txid_results(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = Vec<H256>, Error = Error> + Send>;
    fn transaction_results(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = Vec<Transaction>, Error = Error> + Send>;
    fn transaction_first_result(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = Transaction, Error = Error> + Send>;
    fn transaction_and_receipt_first_result(
        &self,
        query: &QueryId<Ethereum>,
    ) -> Box<dyn Future<Item = TransactionAndReceipt, Error = Error> + Send>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_a_ethereum_transaction_query_with_toaddress_it_serializes_ok() {
        let to_address = Some("8457037fcd80a8650c4692d7fcfc1d0a96b92867".into());
        let query = EthereumQuery::Transaction {
            from_address: None,
            to_address,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"from_address":null,"to_address":"0x8457037fcd80a8650c4692d7fcfc1d0a96b92867","is_contract_creation":null,"transaction_data":null,"transaction_data_length":null}"#
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
            transaction_data_length: None,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"from_address":null,"to_address":null,"is_contract_creation":null,"transaction_data":null,"transaction_data_length":null}"#
        )
    }

    #[test]
    fn given_a_ethereum_block_query_with_min_timestamp_it_serializes_ok() {
        let query = EthereumQuery::Block {
            min_timestamp_secs: 10u32,
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"min_timestamp_secs":10}"#)
    }

    #[test]
    fn transaction_query_with_data_serializes_correctly() {
        let query = EthereumQuery::Transaction {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: Some(Bytes::from(b"hello world!".to_vec())),
            transaction_data_length: Some(12),
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"from_address":null,"to_address":null,"is_contract_creation":null,"transaction_data":"0x68656c6c6f20776f726c6421","transaction_data_length":12}"#)
    }

    #[test]
    fn events_query_without_data_serializes_correctly() {
        let query = EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: None,
                data: None,
                topics: vec![],
            }],
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(
            query,
            r#"{"event_matchers":[{"address":null,"data":null,"topics":[]}]}"#
        )
    }

    #[test]
    fn events_query_with_data_serializes_correctly() {
        let query = EthereumQuery::Event {
            event_matchers: vec![EventMatcher {
                address: Some("8457037fcd80a8650c4692d7fcfc1d0a96b92867".into()),
                data: Some(Bytes::from(vec![1])),
                topics: vec![Some(Topic(
                    "0x0000000000000000000000000000000000000000000000000000000000000001".into(),
                ))],
            }],
        };
        let query = serde_json::to_string(&query).unwrap();
        assert_eq!(query, r#"{"event_matchers":[{"address":"0x8457037fcd80a8650c4692d7fcfc1d0a96b92867","data":"0x01","topics":["0x0000000000000000000000000000000000000000000000000000000000000001"]}]}"#)
    }
}
