use crate::{
    block_processor::{Block, Query, QueryMatchResult, Transaction},
    query_result_repository::QueryResult,
    route_factory::{Error, ExpandResult, QueryParams, QueryType, ShouldExpand},
};
use bitcoin_rpc_client::{BitcoinCoreClient, BitcoinRpcApi};
use bitcoin_support::{
    serialize::BitcoinHash, Address, MinedBlock as BitcoinBlock, OutPoint, SpendsFrom,
    SpendsFromWith, SpendsTo, SpendsWith, Transaction as BitcoinTransaction, TransactionId,
};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BitcoinTransactionQuery {
    pub to_address: Option<Address>,
    pub from_outpoint: Option<OutPoint>,
    pub unlock_script: Option<Vec<Vec<u8>>>,
    #[serde(default = "default_confirmations")]
    confirmations_needed: u32,
}

impl QueryType for BitcoinTransactionQuery {
    fn route() -> &'static str {
        "transactions"
    }
}

impl ShouldExpand for BitcoinTransactionQuery {
    fn should_expand(query_params: &QueryParams) -> bool {
        query_params.expand_results
    }
}

impl ExpandResult for BitcoinTransactionQuery {
    type Client = BitcoinCoreClient;
    type Item = BitcoinTransaction;

    fn expand_result(
        result: &QueryResult,
        client: Arc<BitcoinCoreClient>,
    ) -> Result<Vec<BitcoinTransaction>, Error> {
        let mut expanded_result: Vec<BitcoinTransaction> = Vec::new();
        for tx_id in result.clone().0 {
            let tx_id = TransactionId::from_hex(tx_id.as_str()).map_err(|_| Error::InvalidHex)?;

            let transaction = client
                .get_raw_transaction_verbose(&tx_id)
                .map_err(Error::BitcoinRpcConnection)?
                .map_err(Error::BitcoinRpcResponse)?;
            expanded_result.push(transaction.into());
        }
        Ok(expanded_result)
    }
}

fn default_confirmations() -> u32 {
    1
}

impl Query<BitcoinTransaction> for BitcoinTransactionQuery {
    fn matches(&self, transaction: &BitcoinTransaction) -> QueryMatchResult {
        match self {
            Self {
                to_address,
                from_outpoint,
                unlock_script,
                confirmations_needed,
            } => {
                let mut result = true;

                result = result
                    && match to_address {
                        Some(to_address) => transaction.spends_to(to_address),
                        _ => result,
                    };

                result = result
                    && match (from_outpoint, unlock_script) {
                        (Some(from_outpoint), Some(unlock_script)) => {
                            transaction.spends_from_with(from_outpoint, unlock_script)
                        }
                        (Some(from_outpoint), None) => transaction.spends_from(from_outpoint),
                        (None, Some(unlock_script)) => transaction.spends_with(unlock_script),
                        (..) => result,
                    };

                if result {
                    QueryMatchResult::yes_with_confirmations(*confirmations_needed)
                } else {
                    QueryMatchResult::no()
                }
            }
        }
    }
    fn is_empty(&self) -> bool {
        self.to_address.is_none() && self.from_outpoint.is_none() && self.unlock_script.is_none()
    }
}

impl Transaction for BitcoinTransaction {
    fn transaction_id(&self) -> String {
        self.txid().to_string()
    }
}

impl Block for BitcoinBlock {
    type Transaction = BitcoinTransaction;

    fn blockhash(&self) -> String {
        format!("{:x}", self.as_ref().header.bitcoin_hash())
    }
    fn prev_blockhash(&self) -> String {
        format!("{:x}", self.as_ref().header.prev_blockhash)
    }
    fn transactions(&self) -> &[BitcoinTransaction] {
        self.as_ref().txdata.as_slice()
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BitcoinBlockQuery {
    pub min_height: Option<u32>,
}

impl QueryType for BitcoinBlockQuery {
    fn route() -> &'static str {
        "blocks"
    }
}

impl ShouldExpand for BitcoinBlockQuery {
    fn should_expand(_: &QueryParams) -> bool {
        false
    }
}

impl ExpandResult for BitcoinBlockQuery {
    type Client = ();
    type Item = ();

    fn expand_result(_result: &QueryResult, _client: Arc<()>) -> Result<Vec<Self::Item>, Error> {
        unimplemented!()
    }
}

impl Query<BitcoinBlock> for BitcoinBlockQuery {
    fn matches(&self, block: &BitcoinBlock) -> QueryMatchResult {
        match self.min_height {
            Some(ref height) => {
                if *height <= block.height {
                    QueryMatchResult::yes()
                } else {
                    QueryMatchResult::no()
                }
            }
            None => {
                warn!("min_height not set, nothing to compare");
                QueryMatchResult::no()
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.min_height.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_support::{
        serialize::deserialize, Block, BlockHeader, MinedBlock, Sha256dHash,
        Transaction as BitcoinTransaction,
    };
    use hex;
    use spectral::prelude::*;

    const WITNESS_TX: &'static str = "0200000000010124e06fe5594b941d06c7385dc7307ec694a41f7d307423121855ee17e47e06ad0100000000ffffffff0137aa0b000000000017a914050377baa6e8c5a07aed125d0ef262c6d5b67a038705483045022100d780139514f39ed943179e4638a519101bae875ec1220b226002bcbcb147830b0220273d1efb1514a77ee3dd4adee0e896b7e76be56c6d8e73470ae9bd91c91d700c01210344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f20ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e01015b63a82091d6a24697ed31932537ae598d3de3131e1fcd0641b9ac4be7afcb376386d71e8876a9149f4a0cf348b478336cb1d87ea4c8313a7ca3de1967029000b27576a91465252e57f727a27f32c77098e14d88d8dbec01816888ac00000000";

    fn parse_raw_tx(raw_tx: &str) -> BitcoinTransaction {
        let hex_tx = hex::decode(raw_tx).unwrap();
        let tx: Result<BitcoinTransaction, _> = deserialize(&hex_tx);
        tx.unwrap()
    }

    fn create_unlock_script_stack(data: Vec<&str>) -> Vec<Vec<u8>> {
        data.iter().map(|data| hex::decode(data).unwrap()).collect()
    }

    fn create_outpoint(tx: &str, vout: u32) -> OutPoint {
        OutPoint {
            txid: Sha256dHash::from_hex(tx).unwrap(),
            vout,
        }
    }

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

        let query = BitcoinBlockQuery {
            min_height: Some(42),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::no());
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

        let query = BitcoinBlockQuery {
            min_height: Some(42),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::yes());
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

        let query = BitcoinBlockQuery {
            min_height: Some(42),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::yes());
    }

    #[test]
    fn given_transaction_with_to_then_to_address_query_matches() {
        let tx = parse_raw_tx(WITNESS_TX);

        let query = BitcoinTransactionQuery {
            to_address: Some("329XTScM6cJgu8VZvaqYWpfuxT1eQDSJkP".parse().unwrap()),
            from_outpoint: None,
            unlock_script: None,
            confirmations_needed: 0,
        };

        assert_that(&query.matches(&tx)).is_equal_to(QueryMatchResult::yes());
    }

    #[test]
    fn given_a_witness_transaction_with_unlock_script_then_unlock_script_query_matches() {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "01",
        ]);

        let query = BitcoinTransactionQuery {
            to_address: None,
            from_outpoint: None,
            unlock_script: Some(unlock_script),
            confirmations_needed: 0,
        };

        assert_that(&query.matches(&tx)).is_equal_to(QueryMatchResult::yes());
    }

    #[test]
    fn given_a_witness_transaction_with_different_unlock_script_then_unlock_script_query_wont_match(
    ) {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec!["102030405060708090", "00"]);

        let query = BitcoinTransactionQuery {
            to_address: None,
            from_outpoint: None,
            unlock_script: Some(unlock_script),
            confirmations_needed: 0,
        };

        assert_that(&query.matches(&tx)).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_a_witness_transaction_with_unlock_script_then_spends_from_with_query_match() {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "01",
        ]);
        let outpoint = create_outpoint(
            "ad067ee417ee5518122374307d1fa494c67e30c75d38c7061d944b59e56fe024",
            1u32,
        );

        let query = BitcoinTransactionQuery {
            to_address: None,
            from_outpoint: Some(outpoint),
            unlock_script: Some(unlock_script),
            confirmations_needed: 0,
        };

        assert_that(&query.matches(&tx)).is_equal_to(QueryMatchResult::yes());
    }

}
