use bitcoin_support::{consensus::Decodable, deserialize, Address, BitcoinHash, Block};
use btsieve::{
    bitcoin::TransactionQuery, first_or_else::StreamExt, BlockByHash, LatestBlock,
    MatchingTransactions,
};
use serde::export::fmt::Debug;
use std::{
    collections::HashMap,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::prelude::{Future, FutureExt, IntoFuture};

#[derive(Clone)]
struct BitcoinConnectorMock {
    all_blocks: HashMap<bitcoin_support::BlockId, Block>,
    latest_blocks: Vec<Block>,
    latest_time_return_block: Instant,
    current_latest_block_index: usize,
}

impl BitcoinConnectorMock {
    fn new(latest_blocks: Vec<&Block>, all_blocks: Vec<&Block>) -> Self {
        BitcoinConnectorMock {
            all_blocks: all_blocks
                .into_iter()
                .fold(HashMap::new(), |mut hm, block| {
                    hm.insert(block.bitcoin_hash(), block.clone());
                    hm
                }),
            latest_blocks: latest_blocks.into_iter().cloned().collect(),
            latest_time_return_block: Instant::now(),
            current_latest_block_index: 0,
        }
    }
}

impl LatestBlock for BitcoinConnectorMock {
    type Error = ();
    type Block = bitcoin_support::Block;
    type BlockHash = bitcoin_support::BlockId;

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
    type Block = bitcoin_support::Block;
    type BlockHash = bitcoin_support::BlockId;

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
    let block1 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block/block1.hex"
    ));
    let block2 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block/block2.hex"
    ));
    let block3 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block/block3.hex"
    ));

    let connector =
        BitcoinConnectorMock::new(vec![&block1, &block3], vec![&block1, &block2, &block3]);

    let future = connector
        .matching_transactions(TransactionQuery {
            to_address: Some(
                Address::from_str(
                    include_str!("./test_data/find_transaction_in_missing_block/address").trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!());

    let transaction = wait(future);

    let expected_transaction = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block/transaction.hex"
    ));
    assert_eq!(transaction, expected_transaction);
}

#[test]
fn find_transaction_in_missing_block_with_big_gap() {
    let block1 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block1.hex"
    ));
    let block2 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block2_with_transaction.hex"
    ));
    let block3 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block3.hex"
    ));
    let block4 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block4.hex"
    ));
    let block5 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block5.hex"
    ));
    let block6 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block6.hex"
    ));
    let block7 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block7.hex"
    ));
    let block8 = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/block8.hex"
    ));

    let connector = BitcoinConnectorMock::new(vec![&block1, &block8], vec![
        &block1, &block2, &block3, &block4, &block5, &block6, &block7, &block8,
    ]);

    let future = connector
        .matching_transactions(TransactionQuery {
            to_address: Some(
                Address::from_str(
                    include_str!(
                        "./test_data/find_transaction_in_missing_block_with_big_gap/address"
                    )
                    .trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!());

    let transaction = wait(future);

    let expected_transaction = from_hex(include_str!(
        "./test_data/find_transaction_in_missing_block_with_big_gap/transaction.hex"
    ));
    assert_eq!(transaction, expected_transaction);
}

#[test]
fn find_transaction_if_blockchain_reorganisation() {
    // first block returned by latest_block
    let block1 = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation/block1.hex"
    ));

    let block2_with_transaction = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation/block2_with_transaction.hex"
    ));

    // second block returned by latest block, whose parent we've never seen before
    let block1b_stale = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation/block1b_stale.hex"
    ));

    let connector = BitcoinConnectorMock::new(
        vec![&block1, &block1b_stale, &block2_with_transaction],
        vec![&block1, &block2_with_transaction, &block1b_stale],
    );

    let future = connector
        .matching_transactions(TransactionQuery {
            to_address: Some(
                Address::from_str(
                    include_str!(
                        "./test_data/find_transaction_if_blockchain_reorganisation/address"
                    )
                    .trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!());

    let transaction = wait(future);

    let expected_transaction = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation/transaction.hex"
    ));
    assert_eq!(transaction, expected_transaction);
}

#[test]
fn find_transaction_if_blockchain_reorganisation_with_long_chain() {
    let block1 = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/block1.hex"
    ));

    let block2 = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/block2.hex"
    ));

    let block3 = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/block3.hex"
    ));

    // first block returned by latest_block
    let block4 = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/block4.hex"
    ));

    let block5_with_transaction = from_hex(
        include_str!(
            "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/block5_with_transaction.hex"
        )
        ,
    );

    // second block returned by latest block, whose parent we've never seen before
    let block4b_stale = from_hex(
        include_str!("./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/block4b_stale.hex")
            ,
    );

    let connector = BitcoinConnectorMock::new(
        vec![&block4, &block4b_stale, &block5_with_transaction],
        vec![
            &block1,
            &block2,
            &block3,
            &block4,
            &block5_with_transaction,
            &block4b_stale,
        ],
    );

    let future = connector
        .matching_transactions(TransactionQuery {
            to_address: Some(
                Address::from_str(
                    include_str!(
                        "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/address"
                    ).trim()
                    ,
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!());

    let transaction = wait(future);

    let expected_transaction = from_hex(include_str!(
        "./test_data/find_transaction_if_blockchain_reorganisation_with_long_chain/transaction.hex"
    ));
    assert_eq!(transaction, expected_transaction);
}

fn from_hex<T: Decodable>(hex: &str) -> T {
    let bytes = hex::decode(hex.trim()).unwrap();
    deserialize(bytes.as_slice()).unwrap()
}

fn wait<T: Send + 'static, E: Debug + Send + 'static>(
    future: impl Future<Item = T, Error = E> + Send + 'static,
) -> T {
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    runtime
        .block_on(future.timeout(Duration::from_secs(10)))
        .unwrap()
}
