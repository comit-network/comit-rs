use crate::{
    query_result_repository::QueryResult,
    route_factory::{Error, ExpandResult, QueryParams, QueryType, ShouldExpand},
};
use bitcoin_rpc_client::{BitcoinCoreClient, BitcoinRpcApi};
use bitcoin_support::{
    Address, MinedBlock as Block, OutPoint, SpendsFrom, SpendsFromWith, SpendsTo, SpendsWith,
    Transaction, TransactionId,
};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct TransactionQuery {
    pub to_address: Option<Address>,
    pub from_outpoint: Option<OutPoint>,
    pub unlock_script: Option<Vec<Vec<u8>>>,
}

impl QueryType for TransactionQuery {
    fn route() -> &'static str {
        "transactions"
    }
}

impl ShouldExpand for TransactionQuery {
    fn should_expand(query_params: &QueryParams) -> bool {
        query_params.expand_results
    }
}

impl ExpandResult for TransactionQuery {
    type Client = BitcoinCoreClient;
    type Item = Transaction;

    fn expand_result(
        result: &QueryResult,
        client: Arc<BitcoinCoreClient>,
    ) -> Result<Vec<Transaction>, Error> {
        let mut expanded_result: Vec<Transaction> = Vec::new();
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

impl TransactionQuery {
    pub fn matches(&self, transaction: &Transaction) -> bool {
        match self {
            Self {
                to_address,
                from_outpoint,
                unlock_script,
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

                result
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BlockQuery {
    pub min_height: Option<u32>,
}

impl QueryType for BlockQuery {
    fn route() -> &'static str {
        "blocks"
    }
}

impl ShouldExpand for BlockQuery {
    fn should_expand(_: &QueryParams) -> bool {
        false
    }
}

impl ExpandResult for BlockQuery {
    type Client = ();
    type Item = ();

    fn expand_result(_result: &QueryResult, _client: Arc<()>) -> Result<Vec<Self::Item>, Error> {
        unimplemented!()
    }
}

impl BlockQuery {
    pub fn matches(&self, block: &Block) -> bool {
        self.min_height.map_or_else(
            || {
                warn!("min_height not set, nothing to compare");
                false
            },
            |height| height <= block.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_support::{
        serialize::deserialize, Block, BlockHeader, MinedBlock, Sha256dHash, Transaction,
    };
    use spectral::prelude::*;

    const WITNESS_TX: &'static str = "0200000000010124e06fe5594b941d06c7385dc7307ec694a41f7d307423121855ee17e47e06ad0100000000ffffffff0137aa0b000000000017a914050377baa6e8c5a07aed125d0ef262c6d5b67a038705483045022100d780139514f39ed943179e4638a519101bae875ec1220b226002bcbcb147830b0220273d1efb1514a77ee3dd4adee0e896b7e76be56c6d8e73470ae9bd91c91d700c01210344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f20ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e01015b63a82091d6a24697ed31932537ae598d3de3131e1fcd0641b9ac4be7afcb376386d71e8876a9149f4a0cf348b478336cb1d87ea4c8313a7ca3de1967029000b27576a91465252e57f727a27f32c77098e14d88d8dbec01816888ac00000000";

    fn parse_raw_tx(raw_tx: &str) -> Transaction {
        let hex_tx = hex::decode(raw_tx).unwrap();
        let tx: Result<Transaction, _> = deserialize(&hex_tx);
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

    #[test]
    fn given_transaction_with_to_then_to_address_query_matches() {
        let tx = parse_raw_tx(WITNESS_TX);

        let query = TransactionQuery {
            to_address: Some("329XTScM6cJgu8VZvaqYWpfuxT1eQDSJkP".parse().unwrap()),
            from_outpoint: None,
            unlock_script: None,
        };

        let result = query.matches(&tx);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_a_witness_transaction_with_unlock_script_then_unlock_script_query_matches() {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "01",
        ]);

        let query = TransactionQuery {
            to_address: None,
            from_outpoint: None,
            unlock_script: Some(unlock_script),
        };

        let result = query.matches(&tx);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_a_witness_transaction_with_different_unlock_script_then_unlock_script_query_wont_match(
    ) {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec!["102030405060708090", "00"]);

        let query = TransactionQuery {
            to_address: None,
            from_outpoint: None,
            unlock_script: Some(unlock_script),
        };

        let result = query.matches(&tx);
        assert_that(&result).is_false();
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

        let query = TransactionQuery {
            to_address: None,
            from_outpoint: Some(outpoint),
            unlock_script: Some(unlock_script),
        };

        let result = query.matches(&tx);
        assert_that(&result).is_true();
    }
}
