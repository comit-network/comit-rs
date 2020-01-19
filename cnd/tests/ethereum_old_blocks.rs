pub mod ethereum_helper;

use chrono::NaiveDateTime;
use cnd::{
    btsieve::ethereum::{matching_transaction, TransactionPattern},
    ethereum::{Block, Transaction, TransactionAndReceipt, TransactionReceipt},
};
use ethereum_helper::EthereumConnectorMock;

#[tokio::test]
async fn find_transaction_in_old_block() {
    let block1_with_transaction: Block<Transaction> = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_old_block/block1_with_transaction.json"
    );
    let transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_in_old_block/transaction.json"
    );
    let receipt: TransactionReceipt =
        include_json_test_data!("./test_data/ethereum/find_transaction_in_old_block/receipt.json");
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_old_block/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_old_block/block5.json"
            ),
        ],
        vec![
            block1_with_transaction.clone(),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_old_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_old_block/block3.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_old_block/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_in_old_block/block5.json"
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

    let start_of_swap =
        NaiveDateTime::from_timestamp(block1_with_transaction.timestamp.low_u32() as i64, 0);
    let expected_transaction_and_receipt = matching_transaction(connector, pattern, start_of_swap)
        .await
        .unwrap();

    assert_eq!(expected_transaction_and_receipt, TransactionAndReceipt {
        transaction,
        receipt
    });
}
