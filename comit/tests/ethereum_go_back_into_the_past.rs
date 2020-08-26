pub mod ethereum_helper;

use chrono::{DateTime, NaiveDateTime, Utc};
use comit::{
    btsieve::ethereum::matching_transaction_and_receipt,
    ethereum::{Block, Transaction, TransactionReceipt},
};
use ethereum_helper::EthereumConnectorMock;

#[tokio::test]
async fn find_transaction_go_back_into_the_past() {
    let block1_with_transaction: Block = include_json_test_data!(
        "./test_data/ethereum/find_transaction_go_back_into_the_past/block1_with_transaction.json"
    );
    let want_transaction: Transaction = include_json_test_data!(
        "./test_data/ethereum/find_transaction_go_back_into_the_past/transaction.json"
    );
    let want_receipt: TransactionReceipt = include_json_test_data!(
        "./test_data/ethereum/find_transaction_go_back_into_the_past/receipt.json"
    );
    let connector = EthereumConnectorMock::new(
        vec![
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_go_back_into_the_past/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_go_back_into_the_past/block5.json"
            ),
        ],
        vec![
            block1_with_transaction.clone(),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_go_back_into_the_past/block2.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_go_back_into_the_past/block3.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_go_back_into_the_past/block4.json"
            ),
            include_json_test_data!(
                "./test_data/ethereum/find_transaction_go_back_into_the_past/block5.json"
            ),
        ],
        vec![(want_transaction.hash, want_receipt.clone())],
    );

    let start_of_swap = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(block1_with_transaction.timestamp.low_u32() as i64, 0),
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
