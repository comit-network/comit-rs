use ethbloom::Input;
use ethereum_support::{
    web3::types::{TransactionReceipt, H256},
    Address, Block, Bytes, Transaction,
};

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Topic(pub H256);

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct EventQuery {
    pub event_matchers: Vec<EventMatcher>,
}

/// Event Matcher work similar as web3 filters:
/// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
/// E.g. this `EventMatcher` would match this `Log`:
/// ```rust, ignore
/// EventMatcher {
/// address: 0xe46FB33e4DB653De84cB0E0E8b810A6c4cD39d59,
/// data: None,
/// topics: [
/// None,
/// Some(0x000000000000000000000000e46fb33e4db653de84cb0e0e8b810a6c4cd39d59),
/// None()
/// ],
/// ```
/// ```rust, ignore
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
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct EventMatcher {
    pub address: Option<Address>,
    pub data: Option<Bytes>,
    pub topics: Vec<Option<Topic>>,
}

impl EventQuery {
    pub fn matches_block(&self, block: &Block<Transaction>) -> bool {
        self.event_matchers.iter().all(|log_matcher| {
            log_matcher.topics.iter().all(|topic| {
                topic.as_ref().map_or(true, |topic| {
                    block
                        .logs_bloom
                        .contains_input(Input::Raw(topic.0.as_ref()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_support::web3::types::{
        Address, Block, Bytes, Log, Transaction, TransactionReceipt, H160, H2048, H256,
    };
    use spectral::prelude::*;
    use std::str::FromStr;

    lazy_static::lazy_static! {
        pub static ref REDEEM_BLOOM: H2048 = {
        H2048::from_str(
           "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000000000000000000000000100\
            000000000000000000000000000000000000000000000000000000000000000800000000000000000000000\
            000000000000000000000000000000000000000000000000000000000000000000000000408000000000000\
            000000000000000000000000000000000000000000000000000100000000040000000000000000000000000\
            00000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap()
        };
    }
    lazy_static::lazy_static! {
        pub static ref CONTRACT_ADDRESS: H160 = Address::from_str("e46FB33e4DB653De84cB0E0E8b810A6c4cD39d59").unwrap();
    }
    lazy_static::lazy_static! {
        pub static ref REDEEM_LOG_MSG: H256 = H256::from_str("B8CAC300E37F03AD332E581DEA21B2F0B84EAAADC184A295FEF71E81F44A7413").unwrap();
    }
    lazy_static::lazy_static! {
        pub static ref UNKNOWN_LOG_MSG: H256 = H256::from_str("0000000000000000000000000000000000000000000000000000000000000001").unwrap();
    }

    // unfortunately, Log doesn't derive Default
    fn default_log() -> Log {
        Log {
            address: Default::default(),
            topics: vec![],
            data: Default::default(),
            block_hash: None,
            block_number: None,
            transaction_hash: None,
            transaction_index: None,
            log_index: None,
            transaction_log_index: None,
            log_type: None,
            removed: None,
        }
    }

    impl EventMatcher {
        fn new() -> Self {
            EventMatcher {
                address: None,
                data: None,
                topics: vec![],
            }
        }

        fn for_contract(mut self, address: Address) -> Self {
            self.address = Some(address);
            self
        }

        fn with_topics(mut self, topics: Vec<Option<Topic>>) -> Self {
            self.topics = topics;
            self
        }

        fn for_token_contract_with_transfer_topics() -> Self {
            Self::new()
                .for_contract(*CONTRACT_ADDRESS)
                .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])
        }
    }

    #[test]
    fn given_a_block_with_bloom_filter_should_match_query() {
        let tx = Transaction {
            to: Some(*CONTRACT_ADDRESS),
            ..Transaction::default()
        };
        let block = Block {
            logs_bloom: *REDEEM_BLOOM,
            transactions: vec![tx.clone()],
            ..Block::default()
        };

        let matcher = EventMatcher::for_token_contract_with_transfer_topics();

        let query = EventQuery {
            event_matchers: vec![matcher],
        };

        assert_that!(query.matches_block(&block)).is_true()
    }

    #[test]
    fn given_a_block_without_bloom_filter_should_not_match_query() {
        let tx = Transaction {
            to: Some(*CONTRACT_ADDRESS),
            ..Transaction::default()
        };
        let block = Block {
            logs_bloom: H2048::zero(),
            transactions: vec![tx.clone()],
            ..Block::default()
        };

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
        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG],
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_an_empty_transaction_receipt_should_not_match_query() {
        let matcher = EventMatcher::for_token_contract_with_transfer_topics();

        let query = EventQuery {
            event_matchers: vec![matcher],
        };

        let receipt = TransactionReceipt::default();

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_should_match_two_log_query() {
        let query = EventQuery {
            event_matchers: vec![
                EventMatcher::for_token_contract_with_transfer_topics(),
                EventMatcher::new()
                    .for_contract(*CONTRACT_ADDRESS)
                    .with_topics(vec![Some(Topic(*UNKNOWN_LOG_MSG))]),
            ],
        };

        let log1 = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG],
            data: Bytes::default(),
            ..default_log()
        };
        let log2 = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*UNKNOWN_LOG_MSG],
            data: Bytes::default(),
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log1, log2],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transaction_receipt_with_address_should_not_match_with_different_address() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher::new()
                .for_contract(Address::repeat_byte(1))
                .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])],
        };

        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG],
            data: Bytes::default(),
            ..default_log()
        };
        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transaction_receipt_with_address_should_not_match_with_different_topic() {
        let query = EventQuery {
            event_matchers: vec![EventMatcher::new()
                .for_contract(Address::repeat_byte(1))
                .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])],
        };

        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*UNKNOWN_LOG_MSG],
            data: Bytes::default(),
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }

    #[test]
    fn given_a_transfer_log_should_match_transfer_query() {
        let from_address =
            H256::from_str("00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72")
                .unwrap();
        let to_address =
            H256::from_str("0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .unwrap();

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(*CONTRACT_ADDRESS),
                data: Some(Bytes::from(vec![1, 2, 3])),
                topics: vec![
                    Some(Topic(*REDEEM_LOG_MSG)),
                    Some(Topic(from_address)),
                    Some(Topic(to_address)),
                ],
            }],
        };

        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG, from_address, to_address],
            data: Bytes::from(vec![1, 2, 3]),
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transfer_log_should_match_partial_topics_query() {
        let from_address =
            H256::from_str("00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72")
                .unwrap();
        let to_address =
            H256::from_str("0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .unwrap();

        let query = EventQuery {
            event_matchers: vec![EventMatcher::new()
                .for_contract(*CONTRACT_ADDRESS)
                .with_topics(vec![None, None, Some(Topic(to_address))])],
        };

        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG, from_address, to_address],
            data: Bytes::from(vec![1, 2, 3]),
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_true()
    }

    #[test]
    fn given_a_transfer_log_should_not_match_short_query() {
        let from_address =
            H256::from_str("00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72")
                .unwrap();
        let to_address =
            H256::from_str("0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .unwrap();

        let query = EventQuery {
            event_matchers: vec![EventMatcher {
                address: Some(*CONTRACT_ADDRESS),
                data: None,
                topics: vec![Some(Topic(to_address))],
            }],
        };

        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG, from_address, to_address],
            data: Bytes::from(vec![1, 2, 3]),
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(query.matches_transaction_receipt(receipt)).is_false()
    }
}
