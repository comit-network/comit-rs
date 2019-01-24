use crate::{
    query_result_repository::QueryResult,
    route_factory::{Error, ExpandResult, QueryParams, QueryType, ShouldExpand},
};
use ethbloom::Input;
use ethereum_support::{
    web3::{
        transports::Http,
        types::{TransactionId, TransactionReceipt, H256},
        Web3,
    },
    Address, Block, Bytes, Transaction,
};
use ethereum_types::clean_0x;
use futures::{
    future::Future,
    stream::{self, Stream},
};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Topic(H256);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventQuery {
    event_matchers: Vec<EventMatcher>,
}

/// Event Matcher work similar as web3 filters:
/// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
/// E.g. this `EventMatcher` would match this `Log`:
/// ```
/// EventMatcher {
/// address: 0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59,
/// data: None,
/// topics: [
/// None,
/// Some(0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59),
/// None()
/// ],
/// ```
/// ```
/// Log:
/// [ { address: '0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59',
/// data: '0x123',
/// ..
/// topics:
/// [ '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef',
/// '0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59',
/// '0x000000000000000000000000d51ecee7414c4445534f74208538683702cbb3e4' ],
/// },
/// .. ] //Other data omitted
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct EventMatcher {
    address: Option<Address>,
    data: Option<Bytes>,
    topics: Vec<Option<Topic>>,
}

impl EventQuery {
    pub fn matches_block(&self, block: &Block<Transaction>) -> bool {
        self.event_matchers.iter().all(|log_matcher| {
            log_matcher.topics.iter().all(|topic| {
                topic.as_ref().map_or(true, |topic| {
                    block.logs_bloom.contains_input(Input::Raw(&topic.0))
                })
            })
        })
    }

    pub fn matches_transaction_receipt(&self, transaction_receipt: TransactionReceipt) -> bool {
        self.event_matchers
            .iter()
            .all(|event_matcher| match event_matcher {
                EventMatcher {
                    address: None,
                    data: None,
                    topics,
                } if topics.is_empty() => false,
                EventMatcher {
                    address,
                    data,
                    topics,
                } => transaction_receipt.logs.iter().any(|tx_log| {
                    if address
                        .as_ref()
                        .map_or(false, |address| address != &tx_log.address)
                    {
                        return false;
                    }

                    if data.as_ref().map_or(false, |data| data != &tx_log.data) {
                        return false;
                    }

                    if tx_log.topics.len() == topics.len() {
                        tx_log.topics.iter().enumerate().all(|(index, tx_topic)| {
                            let topic = &topics[index];
                            topic.as_ref().map_or(true, |topic| tx_topic == &topic.0)
                        })
                    } else {
                        false
                    }
                }),
            })
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
    type Item = Transaction;

    // TODO: return TransactionReceipt and not Transaction.
    // Temporarily return the transaction and not the transaction receipt as the
    // secret is currently only available in the transaction call data but not
    // in the receipt. This needs to be fixed with https://github.com/comit-network/comit-rs/issues/638
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

    const REDEEM_BLOOM: &str =
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
         000000000000000000000000000000000000000000000000000000000000000000000000000000000000100\
         000000000000000000000000000000000000000000000000000000000000000800000000000000000000000\
         000000000000000000000000000000000000000000000000000000000000000000000000408000000000000\
         000000000000000000000000000000000000000000000000000100000000040000000000000000000000000\
         00000000000000000000000000000000000000000000000000000000000000000000000000000";

    const EMPTY_BLOOM: &str =
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
         000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
         000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
         000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
         000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
         00000000000000000000000000000000000000000000000000000000000000000000000000000";

    const CONTRACT_ADDRESS: &str = "0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59";
    const REDEEM_LOG_MSG: &str =
        "0xB8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413";
    const UNKNOWN_LOG_MSG: &str =
        "0x0000000000000000000000000000000000000000000000000000000000000001";

    impl EventMatcher {
        fn new() -> Self {
            EventMatcher {
                address: None,
                data: None,
                topics: vec![],
            }
        }

        fn for_contract(mut self, address: Address) -> Self {
            self.address = Some(address.into());
            self
        }

        fn with_topics(mut self, topics: Vec<Option<Topic>>) -> Self {
            self.topics = topics;
            self
        }

        fn for_token_contract_with_transfer_topics() -> Self {
            Self::new()
                .for_contract(CONTRACT_ADDRESS.into())
                .with_topics(vec![Some(Topic(REDEEM_LOG_MSG.into()))])
        }
    }

    #[test]
    fn given_a_block_with_bloom_filter_should_match_query() {
        let tx = transaction(CONTRACT_ADDRESS.into());
        let block = ethereum_block(H2048::from_str(REDEEM_BLOOM).unwrap(), vec![tx.clone()]);

        let matcher = EventMatcher::for_token_contract_with_transfer_topics();

        let query = EventQuery {
            event_matchers: vec![matcher],
        };

        assert_that!(query.matches_block(&block)).is_true()
    }

    #[test]
    fn given_a_block_without_bloom_filter_should_not_match_query() {
        let tx = transaction(CONTRACT_ADDRESS.into());
        let block = ethereum_block(H2048::from_str(EMPTY_BLOOM).unwrap(), vec![tx.clone()]);

        let matcher = EventMatcher::for_token_contract_with_transfer_topics();

        let query = EventQuery {
            event_matchers: vec![matcher],
        };

        assert_that!(query.matches_block(&block)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_should_match_query() {
        let matcher = EventMatcher::for_token_contract_with_transfer_topics();

        let query = EventQuery {
            event_matchers: vec![matcher],
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
        let matcher = EventMatcher::for_token_contract_with_transfer_topics();

        let query = EventQuery {
            event_matchers: vec![matcher],
        };

        let receipt = transaction_receipt(vec![]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_should_match_two_log_query() {
        let query = EventQuery {
            event_matchers: vec![
                EventMatcher::for_token_contract_with_transfer_topics(),
                EventMatcher::new()
                    .for_contract(CONTRACT_ADDRESS.into())
                    .with_topics(vec![Some(Topic(UNKNOWN_LOG_MSG.into()))]),
            ],
        };

        let log1 = log(
            CONTRACT_ADDRESS.into(),
            vec![REDEEM_LOG_MSG.into()],
            Bytes(vec![]),
        );
        let log2 = log(
            CONTRACT_ADDRESS.into(),
            vec![UNKNOWN_LOG_MSG.into()],
            Bytes(vec![]),
        );

        let receipt = transaction_receipt(vec![log1, log2]);

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transaction_receipt_with_address_should_not_match_with_different_address() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher::new()
                .for_contract(1.into())
                .with_topics(vec![Some(Topic(REDEEM_LOG_MSG.into()))])],
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
            event_matchers: vec![EventMatcher::new()
                .for_contract(1.into())
                .with_topics(vec![Some(Topic(REDEEM_LOG_MSG.into()))])],
        };

        let log = log(
            CONTRACT_ADDRESS.into(),
            vec![UNKNOWN_LOG_MSG.into()],
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
                    Some(Topic(REDEEM_LOG_MSG.into())),
                    Some(Topic(from_address.into())),
                    Some(Topic(to_address.into())),
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
            event_matchers: vec![EventMatcher::new()
                .for_contract(CONTRACT_ADDRESS.into())
                .with_topics(vec![None, None, Some(Topic(to_address.into()))])],
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
                topics: vec![Some(Topic(to_address.into()))],
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
}
