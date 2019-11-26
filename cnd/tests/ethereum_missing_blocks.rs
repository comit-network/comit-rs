pub mod ethereum_helper;

use cnd::{
    btsieve::{ethereum::TransactionPattern, MatchingTransactions},
    ethereum::{Transaction, TransactionAndReceipt, TransactionReceipt},
    first_or_else::StreamExt,
};
use ethereum_helper::EthereumConnectorMock;
use tokio::prelude::Future;

#[test]
fn find_transaction_in_missing_block() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/transaction.json"
    );
    let receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block3.json"
            ),
        ],
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block2_with_transaction.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block3.json"
            ),
        ],
        vec![(transaction.hash, receipt.clone())],
        runtime.executor(),
    );

    let expected_transaction_and_receipt: TransactionAndReceipt = connector
        .matching_transactions(
            TransactionPattern {
                from_address: None,
                to_address: Some(transaction.to.unwrap()),
                is_contract_creation: None,
                transaction_data: None,
                transaction_data_length: None,
                events: None,
            },
            None,
        )
        .first_or_else(|| panic!())
        .wait()
        .unwrap();

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}

#[test]
fn find_transaction_in_missing_block_with_big_gap() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/transaction.json"
    );
    let receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_missing_block/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block5.json"
            ),
        ],
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block2_with_transaction.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block3.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_missing_block/block5.json"
            ),
        ],
        vec![(transaction.hash, receipt.clone())],
        runtime.executor(),
    );

    let expected_transaction_and_receipt: TransactionAndReceipt = connector
        .matching_transactions(
            TransactionPattern {
                from_address: None,
                to_address: Some(transaction.to.unwrap()),
                is_contract_creation: None,
                transaction_data: None,
                transaction_data_length: None,
                events: None,
            },
            None,
        )
        .first_or_else(|| panic!())
        .wait()
        .unwrap();

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}
