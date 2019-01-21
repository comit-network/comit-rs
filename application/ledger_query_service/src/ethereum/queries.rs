use crate::{
    query_result_repository::QueryResult,
    route_factory::{Error, ExpandResult, QueryParams, QueryType, ShouldExpand},
    QueryMatchResult,
};
use ethbloom::Input;
use ethereum_support::{
    web3::{
        transports::Http,
        types::{TransactionReceipt, H256, U256},
        Web3,
    },
    Address, Block, Bytes, Transaction, TransactionId,
};
use futures::{
    future::Future,
    stream::{self, Stream},
};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct TransactionQuery {
    from_address: Option<Address>,
    to_address: Option<Address>,
    is_contract_creation: Option<bool>,
    transaction_data: Option<Bytes>,
    transaction_data_length: Option<usize>,
}

type Topic = H256;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventQuery {
    event_matchers: Vec<EventMatcher>,
}

// Event Matcher work similar as web3 filters:
// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
// E.g. this EventMatcher would match this Log:
// EventMatcher {
// address: 0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59,
// data: None,
// topics: [
// None,
// Some(0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59),
// None()
// ],
//
// Log:
// [ { address: '0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59',
// data: '0x123',
// ..
// topics:
// [ '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',
// '0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59',
// '0x000000000000000000000000d51ecee7414c4445534f74208538683702cbb3e4' ],
// },
// .. ] //Other data omitted
// }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct EventMatcher {
    address: Option<Address>,
    data: Option<Bytes>,
    topics: Vec<Option<Topic>>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BlockQuery {
    pub min_timestamp_secs: Option<u64>,
}

impl TransactionQuery {
    pub fn matches(&self, transaction: &Transaction) -> QueryMatchResult {
        match self {
            Self {
                from_address: None,
                to_address: None,
                is_contract_creation: None,
                transaction_data: None,
                transaction_data_length: None,
            } => QueryMatchResult::no(),
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

                if result {
                    QueryMatchResult::yes()
                } else {
                    QueryMatchResult::no()
                }
            }
        }
    }
}

impl QueryType for TransactionQuery {
    fn route() -> &'static str {
        "transactions"
    }
}

impl ShouldExpand for TransactionQuery {
    fn should_expand(params: &QueryParams) -> bool {
        params.expand_results
    }
}

impl ExpandResult for TransactionQuery {
    type Client = Web3<Http>;
    type Item = Transaction;

    fn expand_result(
        result: &QueryResult,
        client: Arc<Web3<Http>>,
    ) -> Result<Vec<Self::Item>, Error> {
        let futures: Vec<_> = result
            .0
            .iter()
            .filter_map(|tx_id| match hex::decode(clean_0x(tx_id)) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    warn!("Skipping {} because it is not valid hex: {:?}", tx_id, e);
                    None
                }
            })
            .map(|id| {
                client
                    .eth()
                    .transaction(TransactionId::Hash(H256::from_slice(id.as_ref())))
                    .map_err(Error::Web3)
            })
            .collect();

        stream::futures_ordered(futures)
            .filter_map(|item| item)
            .collect()
            .wait()
    }
}

fn clean_0x(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    }
}

impl EventQuery {
    pub fn matches_block(&self, block: &Block<Transaction>) -> bool {
        match self {
            Self { event_matchers } if event_matchers.is_empty() => false,
            Self { event_matchers } => event_matchers.iter().all(|log_matcher| {
                log_matcher.topics.iter().all(|topic| {
                    if let Some(topic) = topic {
                        block.logs_bloom.contains_input(Input::Raw(&topic))
                    } else {
                        false
                    }
                })
            }),
        }
    }

    pub fn matches_transaction_receipt(&self, transaction_receipt: TransactionReceipt) -> bool {
        match self {
            Self { event_matchers } if event_matchers.is_empty() => false,
            Self { event_matchers } => event_matchers.iter().all(|event_matcher| {
                return match event_matcher {
                    EventMatcher {
                        address: None,
                        data: None,
                        topics: topics,
                    } if topics.is_empty() => false,
                    EventMatcher {
                        address: address,
                        data: data,
                        topics: topics,
                    } => transaction_receipt.logs.iter().any(|tx_log| {
                        let mut result = true;

                        if let Some(address) = address {
                            result = result && (address == &tx_log.address);
                        }

                        if let Some(data) = data {
                            result = result && (data == &tx_log.data)
                        }

                        if tx_log.topics.len() == topics.len() {
                            tx_log
                                .topics
                                .iter()
                                .enumerate()
                                .for_each(|(index, tx_topic)| {
                                    let topic = topics[index];
                                    if let Some(topic) = topic {
                                        result = result && (tx_topic == &topic);
                                    };
                                });
                        } else {
                            result = false
                        }

                        result
                    }),
                };
            }),
        }
    }
}

impl QueryType for EventQuery {
    fn route() -> &'static str {
        "logs"
    }
}

impl ShouldExpand for EventQuery {
    fn should_expand(params: &QueryParams) -> bool {
        params.expand_results
    }
}

impl ExpandResult for EventQuery {
    type Client = Web3<Http>;
    type Item = TransactionReceipt;

    fn expand_result(
        result: &QueryResult,
        client: Arc<Web3<Http>>,
    ) -> Result<Vec<Self::Item>, Error> {
        let futures: Vec<_> = result
            .0
            .iter()
            .filter_map(|tx_id| match hex::decode(clean_0x(tx_id)) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    warn!("Skipping {} because it is not valid hex: {:?}", tx_id, e);
                    None
                }
            })
            .map(|id| {
                client
                    .eth()
                    .transaction_receipt(H256::from_slice(id.as_ref()))
                    .map_err(Error::Web3)
            })
            .collect();

        stream::futures_ordered(futures)
            .filter_map(|item| item)
            .collect()
            .wait()
    }
}

impl BlockQuery {
    pub fn matches(&self, block: &Block<Transaction>) -> QueryMatchResult {
        match self.min_timestamp_secs {
            Some(min_timestamp_secs) => {
                let min_timestamp_secs = U256::from(min_timestamp_secs);
                if min_timestamp_secs <= block.timestamp {
                    QueryMatchResult::yes()
                } else {
                    QueryMatchResult::no()
                }
            }
            None => {
                warn!("min_timestamp not set, nothing to compare");
                QueryMatchResult::no()
            }
        }
    }
}

impl QueryType for BlockQuery {
    fn route() -> &'static str {
        "blocks"
    }
}

impl ShouldExpand for BlockQuery {
    fn should_expand(_: &QueryParams) -> bool {
        false
    }
}

impl ExpandResult for BlockQuery {
    type Client = ();
    type Item = ();

    fn expand_result(_result: &QueryResult, _client: Arc<()>) -> Result<Vec<Self::Item>, Error> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web3::types::{
        Address, Block, Bytes, Log, Transaction, TransactionReceipt, H160, H2048, H256, U128, U256,
    };
    use spectral::prelude::*;
    use std::str::FromStr;

    fn ethereum_block(bloom: H2048, transactions: Vec<Transaction>) -> Block<Transaction> {
        Block {
            hash: None,
            parent_hash: H256::from(123),
            uncles_hash: H256::from(123),
            author: H160::from(7),
            state_root: H256::from(123),
            transactions_root: H256::from(123),
            receipts_root: H256::from(123),
            number: None,
            gas_used: U256::from(0),
            gas_limit: U256::from(0),
            extra_data: Bytes::from(vec![]),
            logs_bloom: bloom,
            timestamp: U256::from(0),
            difficulty: U256::from(0),
            total_difficulty: U256::from(0),
            seal_fields: vec![],
            uncles: vec![],
            transactions,
            size: None,
        }
    }

    fn transaction(address: Address) -> Transaction {
        Transaction {
            hash: H256::from(0),
            nonce: U256::from(0),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: H160::from(0),
            to: Some(address),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![]),
        }
    }

    fn log(address: Address, topics: Vec<H256>, data: Bytes) -> Log {
        Log {
            address,
            topics,
            data,
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: None,
            removed: Some(false),
        }
    }

    fn transaction_receipt(logs: Vec<Log>) -> TransactionReceipt {
        TransactionReceipt {
            transaction_hash: H256::from(0),
            transaction_index: U128::from(0),
            block_hash: None,
            block_number: None,
            cumulative_gas_used: U256::from(0),
            gas_used: U256::from(0),
            contract_address: None,
            logs,
            status: None,
        }
    }

    const  REDEEM_BLOOM : &str = "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000\
            000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000040800000000000000000000000000000000000000000000000000\
            00000000000001000000000400000000000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000";

    const  EMPTY_BLOOM : &str =
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            0000000000000000000000000000";

    const CONTRACT_ADDRESS: &str = "0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59";
    const REDEEM_LOG_MSG: &str =
        "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413";
    const RANDOM_LOG_MSG: &str =
        "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7412";

    #[test]
    fn given_a_block_with_bloom_filter_should_match_query() {
        let tx = transaction(CONTRACT_ADDRESS.into());
        let block = ethereum_block(H2048::from_str(REDEEM_BLOOM).unwrap(), vec![tx.clone()]);

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: None,
                topics: vec![Some(REDEEM_LOG_MSG.into())],
            }],
        };

        assert_that!(query.matches_block(&block)).is_true()
    }

    #[test]
    fn given_a_block_without_bloom_filter_should_not_match_query() {
        let tx = transaction(CONTRACT_ADDRESS.into());
        let block = ethereum_block(H2048::from_str(EMPTY_BLOOM).unwrap(), vec![tx.clone()]);

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: None,
                topics: vec![Some(REDEEM_LOG_MSG.into())],
            }],
        };

        assert_that!(query.matches_block(&block)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_should_match_query() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: None,
                topics: vec![Some(REDEEM_LOG_MSG.into())],
            }],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![REDEEM_LOG_MSG.into()],
            Bytes(vec![]),
        );
        let receipt = transaction_receipt(vec![log]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_an_empty_transaction_receipt_should_not_match_query() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: None,
                topics: vec![Some(REDEEM_LOG_MSG.into())],
            }],
        };

        let receipt = transaction_receipt(vec![]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_should_not_match_empty_query() {
        let query1 = EventQuery {
            event_matchers: vec![EventMatcher {
                address: None,
                data: None,
                topics: vec![],
            }],
        };

        let log1 = log(
            CONTRACT_ADDRESS.into(),
            vec![REDEEM_LOG_MSG.into()],
            Bytes(vec![]),
        );
        let receipt1 = transaction_receipt(vec![log1]);
        assert_that!(query1.matches_transaction_receipt(receipt1)).is_false();

        let query2 = EventQuery {
            event_matchers: vec![],
        };

        let log2 = log(
            CONTRACT_ADDRESS.into(),
            vec![REDEEM_LOG_MSG.into()],
            Bytes(vec![]),
        );
        let receipt2 = transaction_receipt(vec![log2]);
        assert_that!(query2.matches_transaction_receipt(receipt2)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_should_match_two_log_query() {
        let query = EventQuery {
            event_matchers: vec![
                EventMatcher {
                    address: Some(CONTRACT_ADDRESS.into()),
                    data: None,
                    topics: vec![Some(REDEEM_LOG_MSG.into())],
                },
                EventMatcher {
                    address: Some(CONTRACT_ADDRESS.into()),
                    data: None,
                    topics: vec![Some(RANDOM_LOG_MSG.into())],
                },
            ],
        };

        let log1 = log(
            CONTRACT_ADDRESS.into(),
            vec![REDEEM_LOG_MSG.into()],
            Bytes(vec![]),
        );
        let log2 = log(
            CONTRACT_ADDRESS.into(),
            vec![RANDOM_LOG_MSG.into()],
            Bytes(vec![]),
        );

        let receipt = transaction_receipt(vec![log1, log2]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transaction_receipt_with_address_should_not_match_with_different_address() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(1.into()),
                data: None,
                topics: vec![Some(REDEEM_LOG_MSG.into())],
            }],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![REDEEM_LOG_MSG.into()],
            Bytes(vec![]),
        );
        let receipt = transaction_receipt(vec![log]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_with_address_should_not_match_with_different_topic() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(1.into()),
                data: None,
                topics: vec![Some(REDEEM_LOG_MSG.into())],
            }],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![RANDOM_LOG_MSG.into()],
            Bytes(vec![]),
        );

        let receipt = transaction_receipt(vec![log]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transfer_log_should_match_transfer_query() {
        let from_address = "0x00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72";
        let to_address = "0x0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8";

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: Some(Bytes::from(vec![1, 2, 3])),
                topics: vec![
                    Some(REDEEM_LOG_MSG.into()),
                    Some(from_address.into()),
                    Some(to_address.into()),
                ],
            }],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![
                REDEEM_LOG_MSG.into(),
                from_address.into(),
                to_address.into(),
            ],
            Bytes::from(vec![1, 2, 3]),
        );

        let receipt = transaction_receipt(vec![log]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transfer_log_should_match_partial_topics_query() {
        let from_address = "0x00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72";
        let to_address = "0x0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8";

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: None,
                topics: vec![None, None, Some(to_address.into())],
            }],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![
                REDEEM_LOG_MSG.into(),
                from_address.into(),
                to_address.into(),
            ],
            Bytes::from(vec![1, 2, 3]),
        );

        let receipt = transaction_receipt(vec![log]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transfer_log_should_not_match_short_query() {
        let from_address = "0x00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72";
        let to_address = "0x0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8";

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(CONTRACT_ADDRESS.into()),
                data: None,
                topics: vec![Some(to_address.into())],
            }],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![
                REDEEM_LOG_MSG.into(),
                from_address.into(),
                to_address.into(),
            ],
            Bytes::from(vec![1, 2, 3]),
        );

        let receipt = transaction_receipt(vec![log]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

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
        assert_that(&result).is_equal_to(QueryMatchResult::yes_with_confirmations(0));
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
        assert_that(&result).is_equal_to(QueryMatchResult::no());
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
        assert_that(&result).is_equal_to(QueryMatchResult::yes_with_confirmations(0));
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
        assert_that(&result).is_equal_to(QueryMatchResult::no());
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
        assert_that(&result).is_equal_to(QueryMatchResult::no());
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
        assert_that(&result).is_equal_to(QueryMatchResult::yes_with_confirmations(0));

        let result = query_data_length.matches(&transaction);
        assert_that(&result).is_equal_to(QueryMatchResult::yes_with_confirmations(0));

        let result = refund_query.matches(&transaction);
        assert_that(&result).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_query_transaction_data_is_empty_transaction_matches() {
        let query_data = TransactionQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: Some(Bytes::from(vec![])),
            transaction_data_length: None,
        };

        let query_data_length = TransactionQuery {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: Some(0),
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
            input: Bytes::from(vec![]),
        };

        let result = query_data.matches(&transaction);
        assert_that(&result).is_equal_to(QueryMatchResult::yes_with_confirmations(0));

        let result = query_data_length.matches(&transaction);
        assert_that(&result).is_equal_to(QueryMatchResult::yes_with_confirmations(0))
    }

    #[test]
    fn given_no_conditions_in_query_transaction_fails() {
        let query = TransactionQuery {
            from_address: None,
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
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            value: U256::from(0),
            gas_price: U256::from(0),
            gas: U256::from(0),
            input: Bytes::from(vec![1, 2, 3, 4, 5]),
        };
        let result = query.matches(&transaction);
        assert_that(&result).is_equal_to(QueryMatchResult::no())
    }

}
