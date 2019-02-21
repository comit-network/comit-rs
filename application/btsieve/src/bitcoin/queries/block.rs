use crate::{
    query_result_repository::QueryResult,
    route_factory::{Error, Expand, QueryParams, QueryType, ShouldEmbed},
};
use bitcoin_rpc_client::BitcoinCoreClient;
use bitcoin_support::MinedBlock;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BlockQuery {
    pub min_height: Option<u32>,
}

impl QueryType for BlockQuery {
    fn route() -> &'static str {
        "blocks"
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Embed {}

impl ShouldEmbed<Embed> for BlockQuery {
    fn should_embed(_: &QueryParams<Embed>) -> bool {
        false
    }
}

impl Expand<Embed> for BlockQuery {
    type Client = BitcoinCoreClient;
    type Item = ();

    fn expand(
        _: &QueryResult,
        _: &Vec<Embed>,
        _: Arc<BitcoinCoreClient>,
    ) -> Result<Vec<Self::Item>, Error> {
        unimplemented!()
    }
}

impl BlockQuery {
    pub fn matches(&self, block: &MinedBlock) -> bool {
        self.min_height
            .map_or(true, |height| height <= block.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_support::{Block, BlockHeader, Sha256dHash};
    use spectral::prelude::*;

    #[test]
    fn given_query_min_height_then_lesser_block_does_not_match() {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash: Sha256dHash::default(),
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 1,
            nonce: 0,
        };

        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            40,
        );

        let query = BlockQuery {
            min_height: Some(42),
        };

        let result = query.matches(&block);
        assert_that(&result).is_false();
    }

    #[test]
    fn given_query_min_height_then_exact_block_matches() {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash: Sha256dHash::default(),
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 1,
            nonce: 0,
        };

        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            42,
        );

        let query = BlockQuery {
            min_height: Some(42),
        };

        let result = query.matches(&block);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_query_min_height_then_greater_block_matches() {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash: Sha256dHash::default(),
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 1,
            nonce: 0,
        };

        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            45,
        );

        let query = BlockQuery {
            min_height: Some(42),
        };

        let result = query.matches(&block);
        assert_that(&result).is_true();
    }

}
