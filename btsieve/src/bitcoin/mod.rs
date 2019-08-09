pub mod bitcoind_zmq_listener;
pub mod block_processor;
pub mod queries;

pub use self::{
    block_processor::{check_block_queries, check_transaction_queries},
    queries::{BlockQuery, TransactionQuery},
};
use crate::{Bitcoin, Blockchain};
use bitcoin_support::MinedBlock;

impl Blockchain<MinedBlock> for Bitcoin {
    fn add_block(&mut self, block: MinedBlock) {
        if self.0.nodes.contains(&block) {
            return log::warn!("Block already known {:?} ", block);
        }
        match self.find_predecessor(&block) {
            Some(_prev) => {
                self.0.vertices.push((
                    block.clone().block.header.prev_blockhash,
                    block.clone().block.header.merkle_root,
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

    fn find_predecessor(&self, block: &MinedBlock) -> Option<&MinedBlock> {
        self.0.nodes.iter().find(|b| {
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
    use bitcoin_support::{Block, BlockHeader, FromHex, Sha256dHash};
    use spectral::{option::OptionAssertions, *};

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
        assert_that(&bitcoin_chain.size()).is_equal_to(&1);
    }

    #[test]
    fn add_block_twice_should_ignore_once() {
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
    fn add_block_and_find_predecessor() {
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

        assert_that(&bitcoin_chain.find_predecessor(&block1)).is_none();
        assert_that(&bitcoin_chain.find_predecessor(&block2))
            .is_some()
            .is_equal_to(&block1);
    }
}
