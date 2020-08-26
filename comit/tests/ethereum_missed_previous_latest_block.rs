pub mod ethereum_helper;

use chrono::{DateTime, NaiveDateTime, Utc};
use comit::{
    btsieve::ethereum::matching_transaction_and_receipt,
    ethereum::{Block, Transaction, TransactionReceipt},
};
use ethereum_helper::EthereumConnectorMock;

#[tokio::test]
async fn find_transaction_missed_previous_latest_block_single_block_gap() {
    let want_transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_missed_previous_latest_block/transaction.json"
    );
    let want_receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_missed_previous_latest_block/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block4.json"
            ),
        ],
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block3_with_transaction.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block4.json"
            ),
        ],
        vec![(want_transaction.hash, want_receipt.clone())],
    );
    let block2: Block = include_json_test_data!(
        "./test_data/ethereum/find_transaction_missed_previous_latest_block/block2.json"
    );
    let start_of_swap = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(block2.timestamp.as_u32() as i64, 0),
        Utc,
    );

    let (got_transaction, got_receipt) =
        matching_transaction_and_receipt(&connector, start_of_swap, {
            |transaction| transaction.to == want_transaction.to
        })
        .await
        .expect("failed to get the transaction and receipt");

    assert_eq!(
        (got_transaction, got_receipt),
        (want_transaction, want_receipt)
    );
}

#[tokio::test]
async fn find_transaction_missed_previous_latest_block_two_block_gap() {
    let want_transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_missed_previous_latest_block/transaction.json"
    );
    let want_receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_missed_previous_latest_block/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block5.json"
            ),
        ],
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block1.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block3_with_transaction.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_missed_previous_latest_block/block5.json"
            ),
        ],
        vec![(want_transaction.hash, want_receipt.clone())],
    );
    let block2: Block = include_json_test_data!(
        "./test_data/ethereum/find_transaction_missed_previous_latest_block/block2.json"
    );
    let start_of_swap = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(block2.timestamp.as_u32() as i64, 0),
        Utc,
    );

    let (got_transaction, got_receipt) =
        matching_transaction_and_receipt(&connector, start_of_swap, {
            |transaction| transaction.to == want_transaction.to
        })
        .await
        .expect("failed to get the transaction and receipt");

    assert_eq!(
        (got_transaction, got_receipt),
        (want_transaction, want_receipt)
    );
}
