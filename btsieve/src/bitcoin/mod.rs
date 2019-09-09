pub mod bitcoind_http_blocksource;
pub mod block_processor;
pub mod blockchain_info_bitcoin_http_blocksource;
pub mod queries;

pub use self::{block_processor::check_transaction_queries, queries::TransactionQuery};
use crate::{Bitcoin, Blockchain};
use bitcoin_support::{BitcoinHash, Block};

impl Blockchain<Block> for Bitcoin {
    fn add_block(&mut self, block: Block) {
        if self.0.nodes.contains(&block) {
            return log::warn!("Block already known {:?} ", block);
        }
        let block_hash = block.bitcoin_hash();

        match self.find_predecessor(&block) {
            Some(_prev) => {
                self.0
                    .vertices
                    .push((block.clone().header.prev_blockhash, block_hash));
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
        self.0.nodes.iter().find(|b| {
            let block_hash = b.bitcoin_hash();
            block_hash.eq(&block.header.prev_blockhash)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bitcoin_support::{deserialize, Block, BlockHeader, FromHex, Sha256dHash};
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

        let block1_hex = "0000002006226e46111a0b59caaf126043eb5bbf28c34f3a5e332a1fc7b2b73cf188910f916ab5835dbf97d3848f79b2bf9ca535ee8bafa7a83328448279d15cc8e9b7662450785dffff7f200100000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff03510101ffffffff0200f2052a0100000023210384e9f396a3ed6e0013927e4387d687437e354878354bbd4fd0098ff7d6fb81eeac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000";
        let block2_hex = "0000002037eb36655ef7cf9ff058b37acfde115f6824d711d33577c7bfffe2960fecab77269b50d009000a42a6c201b71ffcea8455ef538b972f788ef5c13b8c6b0f5e7f2550785dffff7f200200000001020000000001010000000000000000000000000000000000000000000000000000000000000000ffffffff03520101ffffffff0200f2052a0100000023210357693690036e8831f2c1cc588eae3d09b0ae58e1fd4aad6e29c96f3467ca85d0ac0000000000000000266a24aa21a9ede2f61c3f71d1defd3fa999dfa36953755c690689799962b48bebd836974e8cf90120000000000000000000000000000000000000000000000000000000000000000000000000";

        let block1: Block = deserialize(hex::decode(block1_hex).unwrap().as_ref()).unwrap();
        let block2: Block = deserialize(hex::decode(block2_hex).unwrap().as_ref()).unwrap();

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
