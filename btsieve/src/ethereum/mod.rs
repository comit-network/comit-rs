pub mod block_processor;
pub mod ethereum_web3_block_poller;
pub mod queries;

pub use self::{
    block_processor::{check_block_queries, check_log_queries, check_transaction_queries},
    queries::{BlockQuery, EventQuery, TransactionQuery},
};
use crate::{Blockchain, Ethereum};
use ethereum_support::{Block, Transaction};

impl Blockchain<Block<Transaction>> for Ethereum {
    fn add_block(&mut self, block: Block<Transaction>) {
        match block.hash {
            None => {
                log::warn!("Block does not have a hash {:?} ", block);
            }
            Some(current_hash) => {
                if self.0.nodes.contains(&block) {
                    return log::warn!("Block already known {:?} ", block);
                }
                match self.find_predecessor(&block) {
                    Some(_prev) => {
                        self.0
                            .vertices
                            .push((block.clone().parent_hash, current_hash));
                    }
                    None => {
                        log::warn!("Could not find previous block for {:?} ", block);
                    }
                }
                self.0.nodes.push(block);
            }
        }
    }

    fn size(&self) -> usize {
        self.0.nodes.len()
    }

    fn find_predecessor(&self, block: &Block<Transaction>) -> Option<&Block<Transaction>> {
        self.0
            .nodes
            .iter()
            .find(|b| b.hash.map_or(false, |b1| block.parent_hash == b1))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::web3::types::{Block, H256};
    use spectral::prelude::*;

    #[test]
    fn inital_size_should_be_zero() {
        let blockchain = Ethereum::default();

        assert_that(&blockchain.size()).is_equal_to(&0);
    }

    #[test]
    fn add_block() {
        let mut blockchain = Ethereum::default();

        let block = Block {
            parent_hash: H256::from(1),
            hash: Some(H256::from(2)),
            ..Block::default()
        };

        blockchain.add_block(block);
        assert_that(&blockchain.size()).is_equal_to(&1);
    }

    #[test]
    fn add_block_without_hash_should_ignore() {
        let mut blockchain = Ethereum::default();

        let block = Block {
            parent_hash: H256::from(1),
            hash: None,
            ..Block::default()
        };

        blockchain.add_block(block);
        assert_that(&blockchain.size()).is_equal_to(&0);
    }

    #[test]
    fn add_same_block_twice_should_ignore_second_one() {
        let mut blockchain = Ethereum::default();

        let block = Block {
            parent_hash: H256::from(1),
            hash: Some(H256::from(2)),
            ..Block::default()
        };
        let block_clone = block.clone();

        blockchain.add_block(block);
        assert_that(&blockchain.size()).is_equal_to(&1);
        blockchain.add_block(block_clone);
        assert_that(&blockchain.size()).is_equal_to(&1);
    }

    #[test]
    fn add_block_and_find_predecessor() {
        let mut blockchain = Ethereum::default();

        let block1 = Block {
            parent_hash: H256::from(1),
            hash: Some(H256::from(2)),
            ..Block::default()
        };
        let block2 = Block {
            parent_hash: H256::from(2),
            hash: Some(H256::from(3)),
            ..Block::default()
        };

        blockchain.add_block(block1.clone());
        assert_that(&blockchain.size()).is_equal_to(&1);
        blockchain.add_block(block2.clone());
        assert_that(&blockchain.size()).is_equal_to(&2);

        assert_that(&blockchain.find_predecessor(&block1)).is_none();
        assert_that(&blockchain.find_predecessor(&block2))
            .is_some()
            .is_equal_to(&block1);
    }
}
