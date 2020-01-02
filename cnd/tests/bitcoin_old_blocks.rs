pub mod bitcoin_helper;

use bitcoin::Address;
use bitcoin_helper::BitcoinConnectorMock;
use cnd::btsieve::bitcoin::{matching_transaction, TransactionPattern};
use std::str::FromStr;

#[tokio::test]
async fn find_transaction_in_old_block() {
    let block1_with_transaction: bitcoin::Block = include_hex!(
        "./test_data/bitcoin/find_transaction_in_old_block/block1_with_transaction.hex"
    );
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block5.hex"),
        ],
        vec![
            block1_with_transaction.clone(),
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block2.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_old_block/block5.hex"),
        ],
    );

    let pattern = TransactionPattern {
        to_address: Some(
            Address::from_str(
                include_str!("test_data/bitcoin/find_transaction_in_old_block/address").trim(),
            )
            .unwrap(),
        ),
        from_outpoint: None,
        unlock_script: None,
    };
    let expected_transaction = matching_transaction(
        connector,
        pattern,
        Some(block1_with_transaction.header.time),
    )
    .await
    .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!("./test_data/bitcoin/find_transaction_in_old_block/transaction.hex")
    );
}
