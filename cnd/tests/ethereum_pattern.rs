pub mod ethereum_helper;

use cnd::{
    btsieve::ethereum::{Event, Topic, TransactionPattern},
    ethereum::{Block, Transaction, TransactionReceipt},
};
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

    assert_that!(pattern.needs_receipts(&block)).is_false();
}
