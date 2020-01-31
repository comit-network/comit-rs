pub mod ethereum_helper;

use comit::{
    btsieve::ethereum::{Event, Topic, TransactionPattern},
    ethereum::{Address, Block, Bytes, Transaction, TransactionReceipt},
};
use spectral::prelude::*;
use std::str::FromStr;

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

fn pattern_matches_block(pattern: TransactionPattern) -> bool {
    let block: Block<Transaction> = include_json_test_data!("./test_data/ethereum/block.json");

    for transaction in block.transactions.into_iter() {
        if pattern.matches(&transaction, None) {
            return true;
        }
    }
    false
}

// `matches()` method is a filter which returns `true` if things match and
// `false` otherwise.  In order to _really_ test that the we successfully match
// we first do a negative test then do an identical positive test.

#[test]
fn invalid_from_address_does_not_match_transaction_pattern() {
    let invalid_from_address = Address::from_str("fffffa1fba5b4804863131145bc27256d3abffff")
        .expect("failed to construct from_address");

    let pattern = TransactionPattern {
        from_address: Some(invalid_from_address),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_false();
}

#[test]
fn valid_from_address_does_match_transaction_pattern() {
    let valid_from_address = Address::from_str("fb303a1fba5b4804863131145bc27256d3ab6692")
        .expect("failed to construct from_address");

    let pattern = TransactionPattern {
        from_address: Some(valid_from_address),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_true();
}

#[test]
fn invalid_to_address_does_not_match_transaction_pattern() {
    let invalid_to_address = Address::from_str("fffffe335b2786520f4c5d706c76c9ee69d0ffff")
        .expect("failed to construct to_address");

    let pattern = TransactionPattern {
        to_address: Some(invalid_to_address),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_false();
}

#[test]
fn valid_to_address_does_match_transaction_pattern() {
    let valid_to_address = Address::from_str("c5549e335b2786520f4c5d706c76c9ee69d0a028")
        .expect("failed to construct to_address");

    let pattern = TransactionPattern {
        to_address: Some(valid_to_address),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_true();
}

#[test]
fn invalid_transaction_data_does_not_match_transaction_pattern() {
    let invalid_transaction_data = "ffff9cbb000000000000000000000000d50fb7d948426633ec126aeea140ce4dd09796820000000000000000000000000000000000000000000000000000000ba43bffff";
    let invalid_transaction_data =
        hex::decode(invalid_transaction_data).expect("failed to decode hex data");
    let invalid_transaction_data = Bytes::from(invalid_transaction_data);

    let pattern = TransactionPattern {
        transaction_data: Some(invalid_transaction_data),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_false();
}

#[test]
fn valid_transaction_data_does_match_transaction_pattern() {
    let valid_transaction_data = "a9059cbb000000000000000000000000d50fb7d948426633ec126aeea140ce4dd09796820000000000000000000000000000000000000000000000000000000ba43b7400";
    let valid_transaction_data =
        hex::decode(valid_transaction_data).expect("failed to decode hex data");
    let valid_transaction_data = Bytes::from(valid_transaction_data);

    let pattern = TransactionPattern {
        transaction_data: Some(valid_transaction_data),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_true();
}

#[test]
fn invalid_transaction_data_length_does_not_match_transaction_pattern() {
    let invalid_transaction_data_length = 999_999;

    let pattern = TransactionPattern {
        transaction_data_length: Some(invalid_transaction_data_length),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_false();
}

#[test]
fn valid_transaction_data_length_does_match_transaction_pattern() {
    let valid_transaction_data = "a9059cbb000000000000000000000000d50fb7d948426633ec126aeea140ce4dd09796820000000000000000000000000000000000000000000000000000000ba43b7400";
    let valid_transaction_data =
        hex::decode(valid_transaction_data).expect("failed to decode hex data");

    let invalid_transaction_data_length = valid_transaction_data.len();

    let pattern = TransactionPattern {
        transaction_data_length: Some(invalid_transaction_data_length),
        ..TransactionPattern::default()
    };

    let result = pattern_matches_block(pattern);
    assert_that!(&result).is_true();
}
