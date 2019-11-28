use ethbloom::Input;
use ethereum_support::{
    web3::types::{TransactionReceipt, H256},
    Address, Block, Bytes, Transaction,
};

#[derive(Clone, Default, Debug, Eq, PartialEq)]
/// If the field is set to Some(foo) then only transactions matching foo are
/// returned. Otherwise, when the field is set to None, no pattern matching is
/// done for this field.
pub struct TransactionPattern {
    pub from_address: Option<Address>,
    pub to_address: Option<Address>,
    pub is_contract_creation: Option<bool>,
    pub transaction_data: Option<Bytes>,
    pub transaction_data_length: Option<usize>,
    pub events: Option<Vec<Event>>,
}

impl TransactionPattern {
    /// Does matching based on patterns in self.  If all fields are None any
    /// transaction matches i.e., returns true.
    pub fn matches(&self, transaction: &Transaction, receipt: &TransactionReceipt) -> bool {
        match self {
            Self {
                from_address,
                to_address,
                is_contract_creation,
                transaction_data,
                transaction_data_length,
                events,
            } => {
                if let Some(from_address) = from_address {
                    if transaction.from != *from_address {
                        return false;
                    }
                }

                if let Some(to_address) = to_address {
                    if transaction.to != Some(*to_address) {
                        return false;
                    }
                }

                if let Some(is_contract_creation) = is_contract_creation {
                    // transaction.to address is None if, and only if, the transaction creates a
                    // contract
                    if transaction.to.is_none() != *is_contract_creation {
                        return false;
                    }
                }

                if let Some(transaction_data) = transaction_data {
                    if transaction.input != *transaction_data {
                        return false;
                    }
                }

                if let Some(transaction_data_length) = transaction_data_length {
                    if transaction.input.0.len() != *transaction_data_length {
                        return false;
                    }
                }

                if let Some(events) = events {
                    if !events_exist_in_receipt(events, receipt) {
                        return false;
                    }
                }

                // If all fields are set to None, any transaction matches.
                true
            }
        }
    }

    pub fn can_skip_block(&self, block: &Block<Transaction>) -> bool {
        match self.events {
            None => false,
            Some(ref events) if events.is_empty() && block.logs_bloom.is_empty() => false,
            Some(ref events) => !events.iter().all(|event| {
                event.topics.iter().all(|topic| {
                    topic.as_ref().map_or(true, |topic| {
                        block
                            .logs_bloom
                            .contains_input(Input::Raw(topic.0.as_ref()))
                    })
                })
            }),
        }
    }
}

fn events_exist_in_receipt(events: &[Event], receipt: &TransactionReceipt) -> bool {
    events.iter().all(|event| match event {
        Event {
            address: None,
            data: None,
            topics,
        } if topics.is_empty() => false,
        Event {
            address,
            data,
            topics,
        } => receipt.logs.iter().any(|tx_log| {
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

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Topic(pub H256);

/// Event  work similar as web3 filters:
/// https://web3js.readthedocs.io/en/1.0/web3-eth-subscribe.html?highlight=filter#subscribe-logs
/// E.g. this `Event` would match this `Log`:
/// ```rust, ignore
/// Event {
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
pub struct Event {
    pub address: Option<Address>,
    pub data: Option<Bytes>,
    pub topics: Vec<Option<Topic>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quickcheck::Quickcheck;
    use ethereum_support::web3::types::{
        Address, Block, Bytes, Log, Transaction, TransactionReceipt, H160, H2048, H256,
    };
    use spectral::prelude::*;
    use std::str::FromStr;

    #[test]
    fn given_pattern_from_arbitrary_address_contract_creation_transaction_matches() {
        fn prop(from_address: Quickcheck<Address>, transaction: Quickcheck<Transaction>) -> bool {
            let from_address = from_address.0;

            let pattern = TransactionPattern {
                from_address: Some(from_address),
                to_address: None,
                is_contract_creation: Some(true),
                transaction_data: None,
                transaction_data_length: None,
                events: None,
            };

            let mut transaction = transaction.0;

            transaction.from = from_address;
            transaction.to = None;

            let receipt = TransactionReceipt::default();

            pattern.matches(&transaction, &receipt)
        }

        quickcheck::quickcheck(prop as fn(Quickcheck<Address>, Quickcheck<Transaction>) -> bool)
    }

    #[test]
    fn given_pattern_from_address_doesnt_match() {
        let from_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let pattern = TransactionPattern {
            from_address: Some(from_address),
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
            events: None,
        };

        let transaction = Transaction {
            from: "a00f2cac7bad9285ecfd59e8860f5b2dffffffff".parse().unwrap(),
            ..Transaction::default()
        };

        let receipt = TransactionReceipt::default();

        let result = pattern.matches(&transaction, &receipt);
        assert_that!(&result).is_false();
    }

    #[test]
    fn given_pattern_to_address_transaction_matches() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let pattern = TransactionPattern {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
            events: None,
        };

        let transaction = Transaction {
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some(to_address),
            ..Transaction::default()
        };

        let receipt = TransactionReceipt::default();

        let result = pattern.matches(&transaction, &receipt);
        assert_that!(&result).is_true();
    }

    #[test]
    fn given_pattern_to_address_transaction_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let pattern = TransactionPattern {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
            events: None,
        };

        let transaction = Transaction {
            from: "0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap(),
            to: Some("0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".parse().unwrap()),
            ..Transaction::default()
        };

        let receipt = TransactionReceipt::default();

        let result = pattern.matches(&transaction, &receipt);
        assert_that!(&result).is_false();
    }

    #[test]
    fn given_pattern_to_address_transaction_with_to_none_doesnt_match() {
        let to_address = "a00f2cac7bad9285ecfd59e8860f5b2d8622e099".parse().unwrap();

        let pattern = TransactionPattern {
            from_address: None,
            to_address: Some(to_address),
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
            events: None,
        };

        let transaction = Transaction {
            to: None,
            ..Transaction::default()
        };

        let receipt = TransactionReceipt::default();

        let result = pattern.matches(&transaction, &receipt);
        assert_that!(&result).is_false();
    }

    #[test]
    fn given_pattern_transaction_data_transaction_matches() {
        let pattern_data = TransactionPattern {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: Some(Bytes::from(vec![1, 2, 3, 4, 5])),
            transaction_data_length: None,
            events: None,
        };

        let pattern_data_length = TransactionPattern {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: Some(5),
            events: None,
        };

        let refund_pattern = TransactionPattern {
            from_address: None,
            to_address: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            is_contract_creation: Some(false),
            transaction_data: Some(Bytes::from(vec![])),
            transaction_data_length: None,
            events: None,
        };

        let transaction = Transaction {
            to: Some("0bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".parse().unwrap()),
            input: Bytes::from(vec![1, 2, 3, 4, 5]),
            ..Transaction::default()
        };

        let receipt = TransactionReceipt::default();

        let result = pattern_data.matches(&transaction, &receipt);
        assert_that!(&result).is_true();

        let result = pattern_data_length.matches(&transaction, &receipt);
        assert_that!(&result).is_true();

        let result = refund_pattern.matches(&transaction, &receipt);
        assert_that!(&result).is_false();
    }

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

    impl Event {
        fn new() -> Self {
            Event {
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
    }

    fn transaction_pattern_from_event(event: Event) -> TransactionPattern {
        TransactionPattern {
            from_address: None,
            to_address: None,
            is_contract_creation: None,
            transaction_data: None,
            transaction_data_length: None,
            events: Some(vec![event]),
        }
    }

    #[test]
    fn given_a_block_without_bloom_filter_can_skip_block() {
        let tx = Transaction {
            to: Some(*CONTRACT_ADDRESS),
            ..Transaction::default()
        };
        let block = Block {
            logs_bloom: H2048::zero(),
            transactions: vec![tx.clone()],
            ..Block::default()
        };

        let event = Event::new()
            .for_contract(*CONTRACT_ADDRESS)
            .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))]);
        let pattern = transaction_pattern_from_event(event);

        assert_that!(pattern.can_skip_block(&block)).is_true();
    }

    #[test]
    fn pattern_event_found_in_receipt() {
        let events = vec![Event::new()
            .for_contract(*CONTRACT_ADDRESS)
            .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])];

        let log = Log {
            address: *CONTRACT_ADDRESS,
            topics: vec![*REDEEM_LOG_MSG],
            ..default_log()
        };

        let receipt = TransactionReceipt {
            logs: vec![log],
            ..TransactionReceipt::default()
        };

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_true();
    }

    #[test]
    fn pattern_events_not_found_in_empty_receipt() {
        let events = vec![Event::new()
            .for_contract(*CONTRACT_ADDRESS)
            .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])];

        let receipt = TransactionReceipt::default();

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_false();
    }

    #[test]
    fn pattern_event_with_two_logs_found_in_receipt() {
        let events = vec![
            Event::new()
                .for_contract(*CONTRACT_ADDRESS)
                .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))]),
            Event::new()
                .for_contract(*CONTRACT_ADDRESS)
                .with_topics(vec![Some(Topic(*UNKNOWN_LOG_MSG))]),
        ];

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

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_true();
    }

    #[test]
    fn pattern_event_not_found_in_receipt_if_address_differs() {
        let events = vec![Event::new()
            .for_contract(Address::repeat_byte(1))
            .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])];

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

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_false();
    }

    #[test]
    fn pattern_event_not_found_in_receipt_if_address_and_topics_differ() {
        let events = vec![Event::new()
            .for_contract(Address::repeat_byte(1))
            .with_topics(vec![Some(Topic(*REDEEM_LOG_MSG))])];

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

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_false();
    }

    #[test]
    fn pattern_transfer_log_event_found_in_receipt() {
        let from_address =
            H256::from_str("00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72")
                .unwrap();
        let to_address =
            H256::from_str("0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .unwrap();

        let events = vec![Event {
            address: Some(*CONTRACT_ADDRESS),
            data: Some(Bytes::from(vec![1, 2, 3])),
            topics: vec![
                Some(Topic(*REDEEM_LOG_MSG)),
                Some(Topic(from_address)),
                Some(Topic(to_address)),
            ],
        }];

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

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_true();
    }

    #[test]
    fn pattern_event_with_partial_topics_found_in_receipt() {
        let from_address =
            H256::from_str("00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72")
                .unwrap();
        let to_address =
            H256::from_str("0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .unwrap();

        let events = vec![Event::new()
            .for_contract(*CONTRACT_ADDRESS)
            .with_topics(vec![None, None, Some(Topic(to_address))])];

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

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_true();
    }

    #[test]
    fn pattern_event_with_fewer_topics_not_found_in_receipt() {
        let from_address =
            H256::from_str("00000000000000000000000000a329c0648769a73afac7f9381e08fb43dbea72")
                .unwrap();
        let to_address =
            H256::from_str("0000000000000000000000000A81e8be41b21f651a71aaB1A85c6813b8bBcCf8")
                .unwrap();

        let events = vec![Event {
            address: Some(*CONTRACT_ADDRESS),
            data: None,
            topics: vec![Some(Topic(to_address))],
        }];

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

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_false();
    }

    #[test]
    fn given_pattern_without_events_cannot_skip_block() {
        let block = Block::default();
        let pattern = TransactionPattern {
            events: None,
            ..TransactionPattern::default()
        };

        assert_that!(pattern.can_skip_block(&block)).is_false();
    }

    #[test]
    fn given_pattern_with_empty_events_and_block_with_no_events_cannot_skip_block() {
        let block = Block::default();
        let pattern = TransactionPattern {
            events: Some(Vec::new()),
            ..TransactionPattern::default()
        };

        assert_that!(pattern.can_skip_block(&block)).is_false();
    }

    #[test]
    fn events_found_in_receipt_returns_true_for_empty_events() {
        let events = Vec::new();
        let receipt = TransactionReceipt::default();

        assert_that!(events_exist_in_receipt(&events, &receipt)).is_true();
    }
}
