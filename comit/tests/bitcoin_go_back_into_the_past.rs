pub mod bitcoin_helper;

use bitcoin::Address;
use bitcoin_helper::BitcoinConnectorMock;
use chrono::{DateTime, NaiveDateTime, Utc};
use comit::btsieve::bitcoin::watch_for_created_outpoint;
use std::str::FromStr;

#[tokio::test]
async fn find_transaction_go_back_into_the_past() {
    let block1_with_transaction: bitcoin::Block = include_hex!(
        "./test_data/bitcoin/find_transaction_go_back_into_the_past/block1_with_transaction.hex"
    );
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block5.hex"),
        ],
        vec![
            block1_with_transaction.clone(),
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block2.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/block5.hex"),
        ],
    );

    let start_of_swap = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp(block1_with_transaction.header.time as i64, 0),
        Utc,
    );
    let (expected_transaction, _out_point) = watch_for_created_outpoint(
        &connector,
        start_of_swap,
        Address::from_str(
            include_str!("test_data/bitcoin/find_transaction_go_back_into_the_past/address").trim(),
        )
        .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!("./test_data/bitcoin/find_transaction_go_back_into_the_past/transaction.hex")
    );
}
