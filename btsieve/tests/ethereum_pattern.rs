use btsieve::ethereum::{Event, Topic, TransactionPattern};
use ethereum_support::{Block, Transaction, TransactionReceipt};
use spectral::prelude::*;

#[test]
fn cannot_skip_block_containing_transaction_with_event() {
    let block: Block<Transaction> = include_json_test_data!("./test_data/ethereum/block.json");
    let receipt: TransactionReceipt = include_json_test_data!("./test_data/ethereum/receipt.json");

    let pattern = TransactionPattern {
        from_address: None,
        to_address: None,
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: Some(vec![Event {
            address: Some(receipt.logs[0].address),
            data: None,
            topics: vec![
                Some(Topic(receipt.logs[0].topics[0])),
                Some(Topic(receipt.logs[0].topics[1])),
                Some(Topic(receipt.logs[0].topics[2])),
            ],
        }]),
    };

    assert_that!(pattern.can_skip_block(&block)).is_false();
}

#[macro_export]
macro_rules! include_json_test_data {
    ($file:expr) => {
        serde_json::from_str(include_str!($file)).expect("failed to deserialize test_data")
    };
}
