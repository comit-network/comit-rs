pub mod bitcoin_helper;

use bitcoin::Address;
use bitcoin_helper::BitcoinConnectorMock;
use chrono::{offset::Utc, DateTime, NaiveDateTime};
use comit::btsieve::bitcoin::watch_for_created_outpoint;
use std::str::FromStr;

#[tokio::test]
async fn find_transaction_missed_previous_latest_block() {
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block/block3.hex"),
        ],
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block/block1.hex"),
            include_hex!(
                "./test_data/bitcoin/find_transaction_missed_previous_latest_block/block2_with_transaction.hex"
            ),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block/block3.hex"),
        ],
    );

    let block1: bitcoin::Block = include_hex!(
        "./test_data/bitcoin/find_transaction_missed_previous_latest_block/block1.hex"
    );

    // set the start of the swap to one second after the first block,
    // otherwise we run into the problem, that we try to fetch blocks prior to the
    // first one
    let start_of_swap = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp((block1.header.time as i64) + 1, 0),
        Utc,
    );
    let (expected_transaction, _out_point) = watch_for_created_outpoint(
        &connector,
        start_of_swap,
        Address::from_str(
            include_str!("test_data/bitcoin/find_transaction_missed_previous_latest_block/address")
                .trim(),
        )
        .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
            "./test_data/bitcoin/find_transaction_missed_previous_latest_block/transaction.hex"
        )
    );
}

#[tokio::test]
async fn find_transaction_missed_previous_latest_block_with_big_gap() {
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block8.hex"),
        ],
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block2_with_transaction.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block5.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block6.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block7.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block8.hex"),
        ],
    );

    let block1: bitcoin::Block = include_hex!(
        "./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/block1.hex"
    );

    // set the start of the swap to one second after the first block,
    // otherwise we run into the problem, that we try to fetch blocks prior to the
    // first one
    let start_of_swap = DateTime::<Utc>::from_utc(
        NaiveDateTime::from_timestamp((block1.header.time as i64) + 1, 0),
        Utc,
    );
    let (expected_transaction, _out_point) = watch_for_created_outpoint(
        &connector,
        start_of_swap,
        Address::from_str(
            include_str!(
                "test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/address"
        )
            .trim(),
        )
        .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
            "./test_data/bitcoin/find_transaction_missed_previous_latest_block_with_big_gap/transaction.hex"
        )
    );
}

#[tokio::test]
async fn find_transaction_if_blockchain_reorganisation() {
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/block1b_stale.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/block2_with_transaction.hex"),
        ],
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/block2_with_transaction.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/block1b_stale.hex"),
        ],
    );

    let start_of_swap = Utc::now();
    let (expected_transaction, _out_point) = watch_for_created_outpoint(
        &connector,
        start_of_swap,
        Address::from_str(
            include_str!("test_data/bitcoin/find_transaction_if_blockchain_reorganisation/address")
                .trim(),
        )
        .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
            "./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/transaction.hex"
        )
    );
}

#[tokio::test]
async fn find_transaction_if_blockchain_reorganisation_with_long_chain() {
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block4b_stale.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block5_with_transaction.hex")
        ],
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block2.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block5_with_transaction.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/block4b_stale.hex"),
        ],
    );

    let start_of_swap = Utc::now();
    let (expected_transaction, _out_point) = watch_for_created_outpoint(&connector, start_of_swap, Address::from_str(
        include_str!(
            "test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/address"
        ).trim()
        ,
    )
        .unwrap(),)
        .await
        .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
        "./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/transaction.hex"
    )
    );
}
