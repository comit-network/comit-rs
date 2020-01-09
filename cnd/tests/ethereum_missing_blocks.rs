pub mod ethereum_helper;

use cnd::{
    btsieve::ethereum::{matching_transaction, TransactionPattern},
    ethereum::{Transaction, TransactionAndReceipt, TransactionReceipt},
};
use ethereum_helper::EthereumConnectorMock;

#[tokio::test]
async fn find_transaction_in_missing_block() {
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
    );

    let pattern = TransactionPattern {
        from_address: None,
        to_address: Some(transaction.to.unwrap()),
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: None,
    };
    let expected_transaction_and_receipt = matching_transaction(connector, pattern, None).await;

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}

#[tokio::test]
async fn find_transaction_in_missing_block_with_big_gap() {
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
    );

    let pattern = TransactionPattern {
        from_address: None,
        to_address: Some(transaction.to.unwrap()),
        is_contract_creation: None,
        transaction_data: None,
        transaction_data_length: None,
        events: None,
    };
    let expected_transaction_and_receipt = matching_transaction(connector, pattern, None).await;

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}
