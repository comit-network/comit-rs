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
use tokio::prelude::{future, Future, FutureExt, IntoFuture};

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
        // if the genesis block is reached return a future that never resolves so
        // that we can model the behaviour of considering a chain of blocks that
        // will definitely not contain the transaction specified in the query
        // i.e. due to a blockchain reorganisation
        if block_hash
            == from_hex("0000000000000000000000000000000000000000000000000000000000000000")
        {
            return Box::new(future::empty());
        }

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
    let block1 = from_hex("00000020c9519da8f9ce4a3a050f045f04614c0bf2ffbfa427e00f31ff4b93e236594b730b0e7f730d2c13409204595b7eba7de9d5573773572dcc8bb33fe1206ab57bd4e280915dffff7f200000000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401650101ffffffff0200f2052a01000000232102a9c3de851ca73fb97b49c4f5c53297cee8106800484afc656886f8d9dcfa7334ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000");
    let block2 = from_hex("000000207b2514f92f1535a99da1acc74003bedb065d82ea51c116392f25f8bba1f08f3fdeaee96d3123d824a056af54783d7c9bedf746f57894f1d763a9f6821920acb8fc80915dffff7f200000000002020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401660101ffffffff029c00062a0100000023210264dfefab4ad35855af5070b81a18ee070256ffdac3eefc98780061cb16b77211ac0000000000000000266a24aa21a9ed253d17b47470998c015d8f1c8d25be9a558a9e488bd67484278688ad1705256b01200000000000000000000000000000000000000000000000000000000000000000000000000200000001fa456ea2584ca4ae735288fd797a59d5816132b22b284bb6210b797b6f16f332000000004847304402206196fe7cda1f1b7fccf98e42e8acc6a17a4459b35311d28173f40858868876bb0220540e17cadd26f17340b473917c81001f79a0aff103fa9901297830e001005d7f01feffffff0200e1f5050000000017a914ef5d02af9adce39e8d28c19043d087432b1d7d1587640210240100000017a9141e15086742fde2656b3f82ca18ed624663723b328733000000");
    let block3 = from_hex("00000020444094dfdd4f8f5fdfbfb169229961411ccdef3ca378581e18ce4789fa8ffb29f0f54162748b6ce5cd71703d110db392eb1938c40d220704c826ce429a75bc96ff80915dffff7f200100000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff0401670101ffffffff0200f2052a01000000232103a663aef1d1576347c63446c32a8eabe288da3bb879163e5cf7b0cbd6b5ed831eac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000");

    let connector =
        BitcoinConnectorMock::new(vec![&block1, &block3], vec![&block1, &block2, &block3]);

    let future = connector
        .matching_transactions(TransactionQuery {
            to_address: Some(Address::from_str("2NF4rucYkUXJmZ5NKwjtpZJRoFJJrRPvMcV").unwrap()),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!());

    let transaction = wait(future);

    let expected_transaction = from_hex("0200000001fa456ea2584ca4ae735288fd797a59d5816132b22b284bb6210b797b6f16f332000000004847304402206196fe7cda1f1b7fccf98e42e8acc6a17a4459b35311d28173f40858868876bb0220540e17cadd26f17340b473917c81001f79a0aff103fa9901297830e001005d7f01feffffff0200e1f5050000000017a914ef5d02af9adce39e8d28c19043d087432b1d7d1587640210240100000017a9141e15086742fde2656b3f82ca18ed624663723b328733000000");
    assert_eq!(transaction, expected_transaction);
}

#[test]
fn find_transaction_if_blockchain_reorganisation() {
    // first block returned by latest_block
    let block_query_genesis = from_hex(
        include_str!("./find_transaction_if_blockchain_reorganisation/query_genesis_block.hex")
            .trim(),
    );

    let block_containing_transaction = from_hex(
        include_str!("./find_transaction_if_blockchain_reorganisation/transaction_block.hex")
            .trim(),
    );

    // second block returned by latest block, whose parent we've never seen before
    let stale_block = from_hex(
        include_str!("./find_transaction_if_blockchain_reorganisation/stale_block.hex").trim(),
    );

    let connector = BitcoinConnectorMock::new(
        vec![
            &block_query_genesis,
            &stale_block,
            &block_containing_transaction,
        ],
        vec![
            &block_query_genesis,
            &stale_block,
            &block_containing_transaction,
        ],
    );

    let future = connector
        .matching_transactions(TransactionQuery {
            to_address: Some(
                Address::from_str(
                    include_str!("./find_transaction_if_blockchain_reorganisation/address").trim(),
                )
                .unwrap(),
            ),
            from_outpoint: None,
            unlock_script: None,
        })
        .first_or_else(|| panic!());

    let transaction = wait(future);

    let expected_transaction = from_hex(
        include_str!("./find_transaction_if_blockchain_reorganisation/transaction.hex").trim(),
    );
    assert_eq!(transaction, expected_transaction);
}

fn from_hex<T: Decodable>(hex: &str) -> T {
    let bytes = hex::decode(hex).unwrap();
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
