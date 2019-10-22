use bitcoin::{
    consensus::{deserialize, Decodable},
    hashes::sha256d,
    util::hash::BitcoinHash,
    Address,
};
use btsieve::{
    bitcoin::TransactionPattern, first_or_else::StreamExt, BlockByHash, LatestBlock,
    MatchingTransactions,
};
use std::{
    collections::HashMap,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::prelude::{Future, IntoFuture};

#[derive(Clone)]
struct BitcoinConnectorMock {
    all_blocks: HashMap<sha256d::Hash, bitcoin::Block>,
    latest_blocks: Vec<bitcoin::Block>,
    latest_time_return_block: Instant,
    current_latest_block_index: usize,
}

impl BitcoinConnectorMock {
    fn new(
        latest_blocks: impl IntoIterator<Item = bitcoin::Block>,
        all_blocks: impl IntoIterator<Item = bitcoin::Block>,
    ) -> Self {
        BitcoinConnectorMock {
            all_blocks: all_blocks
                .into_iter()
                .fold(HashMap::new(), |mut hm, block| {
                    hm.insert(block.bitcoin_hash(), block);
                    hm
                }),
            latest_blocks: latest_blocks.into_iter().collect(),
            latest_time_return_block: Instant::now(),
            current_latest_block_index: 0,
        }
    }
}

impl LatestBlock for BitcoinConnectorMock {
    type Error = ();
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn latest_block(
        &mut self,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        if self.latest_blocks.is_empty() {
            return Box::new(Err(()).into_future());
        }

        let latest_block = self.latest_blocks[self.current_latest_block_index].clone();
        if self.latest_time_return_block.elapsed() >= Duration::from_secs(1) {
            self.latest_time_return_block = Instant::now();
            if self
                .latest_blocks
                .get(self.current_latest_block_index + 1)
                .is_some()
            {
                self.current_latest_block_index += 1;
            }
        }
        Box::new(Ok(latest_block).into_future())
    }
}

impl BlockByHash for BitcoinConnectorMock {
    type Error = ();
    type Block = bitcoin::Block;
    type BlockHash = sha256d::Hash;

    fn block_by_hash(
        &self,
        block_hash: Self::BlockHash,
    ) -> Box<dyn Future<Item = Self::Block, Error = Self::Error> + Send + 'static> {
        Box::new(
            self.all_blocks
                .get(&block_hash)
                .cloned()
                .ok_or(())
                .into_future(),
        )
    }
}

#[test]
fn find_transaction_in_missing_block() {
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block/block3.hex"),
        ],
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block/block2.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block/block3.hex"),
        ],
    );

    let expected_transaction: bitcoin::Transaction = connector
        .matching_transactions(TransactionPattern {
            to_address: Some(
                Address::from_str(
                    include_str!("./test_data/bitcoin/find_transaction_in_missing_block/address")
                        .trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!())
        .wait()
        .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!("./test_data/bitcoin/find_transaction_in_missing_block/transaction.hex")
    );
}

#[test]
fn find_transaction_in_missing_block_with_big_gap() {
    let connector = BitcoinConnectorMock::new(
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block8.hex"),
        ],
        vec![
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block1.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block2_with_transaction.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block3.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block4.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block5.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block6.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block7.hex"),
            include_hex!("./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/block8.hex"),
        ],
    );

    let expected_transaction: bitcoin::Transaction = connector
        .matching_transactions(TransactionPattern {
            to_address: Some(
                Address::from_str(
                    include_str!(
                        "./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/address"
                    )
                    .trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!())
        .wait()
        .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
            "./test_data/bitcoin/find_transaction_in_missing_block_with_big_gap/transaction.hex"
        )
    );
}

#[test]
fn find_transaction_if_blockchain_reorganisation() {
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

    let expected_transaction: bitcoin::Transaction = connector
        .matching_transactions(TransactionPattern {
            to_address: Some(
                Address::from_str(
                    include_str!(
                        "./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/address"
                    )
                    .trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!())
        .wait()
        .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
            "./test_data/bitcoin/find_transaction_if_blockchain_reorganisation/transaction.hex"
        )
    );
}

#[test]
fn find_transaction_if_blockchain_reorganisation_with_long_chain() {
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

    let expected_transaction: bitcoin::Transaction = connector
        .matching_transactions(TransactionPattern {
            to_address: Some(
                Address::from_str(
                    include_str!(
                        "./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/address"
                    ).trim()
                    ,
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!())
        .wait()
        .unwrap();

    assert_eq!(
        expected_transaction,
        include_hex!(
        "./test_data/bitcoin/find_transaction_if_blockchain_reorganisation_with_long_chain/transaction.hex"
    )
    );
}

#[macro_export]
macro_rules! include_hex {
    ($file:expr) => {
        from_hex(include_str!($file))
    };
}

fn from_hex<T: Decodable>(hex: &str) -> T {
    let bytes = hex::decode(hex.trim()).unwrap();
    deserialize(bytes.as_slice()).unwrap()
}
