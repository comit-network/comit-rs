pub mod bitcoind_zmq_listener;
pub mod block_processor;
pub mod blockchain_info_bitcoin_http_blocksource;
pub mod queries;

pub use self::{block_processor::check_transaction_queries, queries::TransactionQuery};
use crate::{Bitcoin, Blockchain};
use bitcoin_support::Block;

impl Blockchain<Block> for Bitcoin {
    fn add_block(&mut self, block: Block) {
        if self.0.nodes.contains(&block) {
            return log::warn!("Block already known {:?} ", block);
        }
        match self.find_predecessor(&block) {
            Some(_prev) => {
                self.0.vertices.push((
                    block.clone().header.prev_blockhash,
                    block.clone().header.merkle_root,
                ));
            }
            None => {
                log::warn!("Could not find previous block for {:?} ", block);
            }
        }
        self.0.nodes.push(block);
    }

    fn size(&self) -> usize {
        self.0.nodes.len()
    }

    fn find_predecessor(&self, block: &Block) -> Option<&Block> {
        self.0
            .nodes
            .iter()
            .find(|b| b.header.merkle_root.eq(&block.header.prev_blockhash))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::{Block, BlockHeader, FromHex, Sha256dHash};
    use spectral::{option::OptionAssertions, *};

    fn new_block(prev_blockhash: Sha256dHash, merkle_root: Sha256dHash) -> Block {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash,
            merkle_root,
            time: 0,
            bits: 1,
            nonce: 0,
        };
        Block {
            header: block_header,
            txdata: vec![],
        }
    }

    #[test]
    fn add_block() {
        let mut bitcoin_chain = Bitcoin::default();

        let block = new_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );

        assert_that(&bitcoin_chain.size()).is_equal_to(&0);
        bitcoin_chain.add_block(block);
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
    }

    #[test]
    fn add_block_twice_should_ignore_once() {
        let mut bitcoin_chain = Bitcoin::default();

        let block = new_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );
        let block2 = new_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );

        assert_that(&bitcoin_chain.size()).is_equal_to(&0);
        bitcoin_chain.add_block(block);
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
        bitcoin_chain.add_block(block2);
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
    }

    #[test]
    fn add_block_and_find_predecessor() {
        let mut bitcoin_chain = Bitcoin::default();

        let block1 = new_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
        );

        let block2 = new_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000003",
            )
            .unwrap(),
        );

        assert_that(&bitcoin_chain.size()).is_equal_to(&0);
        bitcoin_chain.add_block(block1.clone());
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
        bitcoin_chain.add_block(block2.clone());
        assert_that(&bitcoin_chain.size()).is_equal_to(&2);

        assert_that(&bitcoin_chain.find_predecessor(&block1)).is_none();
        assert_that(&bitcoin_chain.find_predecessor(&block2))
            .is_some()
            .is_equal_to(&block1);
    }
}
