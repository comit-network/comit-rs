pub mod bitcoind_zmq_listener;
pub mod block_processor;
pub mod queries;

pub use self::{
    block_processor::{check_block_queries, check_transaction_queries},
    queries::{BlockQuery, TransactionQuery},
};
use crate::{AddBlocks, Bitcoin};
use bitcoin_support::MinedBlock;

impl AddBlocks<MinedBlock> for Bitcoin {
    fn add_block(&mut self, block: MinedBlock) {
        if self.contains_precessor(&block) {
            log::warn!("Could not find previous block of {:?} ", block);
        }
        if self.0.nodes.contains(&block) {
            log::warn!("Block already known {:?} ", block);
        } else {
            self.0.nodes.push(block);
        }
    }

    fn size(&self) -> usize {
        self.0.nodes.len()
    }

    fn contains_precessor(&self, block: &MinedBlock) -> bool {
        self.0.nodes.iter().any(|b| {
            b.block
                .header
                .merkle_root
                .eq(&block.block.header.prev_blockhash)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::BlockchainDAG;
    use bitcoin_support::{Block, BlockHeader, FromHex, Sha256dHash};
    use spectral::{boolean::BooleanAssertions, *};

    fn new_mined_block(
        prev_blockhash: Sha256dHash,
        merkle_root: Sha256dHash,
        height: u32,
    ) -> MinedBlock {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash,
            merkle_root,
            time: 0,
            bits: 1,
            nonce: 0,
        };
        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            height,
        );
        block
    }

    #[test]
    fn add_block() {
        let mut bitcoin_chain = Bitcoin::default();

        let block = new_mined_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            1,
        );

        assert_that(&bitcoin_chain.size()).is_equal_to(&0);
        bitcoin_chain.add_block(block);
    }

    #[test]
    fn add_block_twice() {
        let mut bitcoin_chain = Bitcoin::default();

        let block = new_mined_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            1,
        );
        let block2 = new_mined_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            1,
        );

        assert_that(&bitcoin_chain.size()).is_equal_to(&0);
        bitcoin_chain.add_block(block);
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
        bitcoin_chain.add_block(block2);
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
    }

    #[test]
    fn add_block_and_precessor() {
        let mut bitcoin_chain = Bitcoin::default();

        let block1 = new_mined_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            1,
        );

        let block2 = new_mined_block(
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            Sha256dHash::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000003",
            )
            .unwrap(),
            1,
        );

        assert_that(&bitcoin_chain.size()).is_equal_to(&0);
        bitcoin_chain.add_block(block1.clone());
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
        bitcoin_chain.add_block(block2.clone());
        assert_that(&bitcoin_chain.size()).is_equal_to(&2);

        assert_that(&bitcoin_chain.contains_precessor(&block1)).is_false();
        assert_that(&bitcoin_chain.contains_precessor(&block2)).is_true();
    }
}
