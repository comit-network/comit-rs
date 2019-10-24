use crate::bitcoin::transaction_ext::TransactionExt;
use ::bitcoin::{Address, OutPoint, Transaction};

#[derive(Clone, Default, Debug, Eq, PartialEq)]
/// If the field is set to Some(foo) then only transactions matching foo are
/// returned. Otherwise, when the field is set to None, no pattern matching is
/// done for this field.
pub struct TransactionPattern {
    pub to_address: Option<Address>,
    pub from_outpoint: Option<OutPoint>,
    pub unlock_script: Option<Vec<Vec<u8>>>,
}

impl TransactionPattern {
    /// Does matching based on patterns in self.  If all fields are None any
    /// transaction matches i.e., returns true.
    pub fn matches(&self, transaction: &Transaction) -> bool {
        match self {
            Self {
                to_address,
                from_outpoint,
                unlock_script,
            } => {
                if let Some(to_address) = to_address {
                    if !transaction.spends_to(to_address) {
                        return false;
                    }
                }

                match (from_outpoint, unlock_script) {
                    (Some(from_outpoint), Some(unlock_script)) => {
                        if !transaction.spends_from_with(from_outpoint, unlock_script) {
                            return false;
                        }
                    }
                    (Some(from_outpoint), None) => {
                        if !transaction.spends_from(from_outpoint) {
                            return false;
                        }
                    }
                    (None, Some(unlock_script)) => {
                        if !transaction.spends_with(unlock_script) {
                            return false;
                        }
                    }
                    (None, None) => return true,
                };

                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::{
        consensus::deserialize,
        hashes::{hex::FromHex, sha256d},
    };
    use spectral::prelude::*;

    const WITNESS_TX: & str = "0200000000010124e06fe5594b941d06c7385dc7307ec694a41f7d307423121855ee17e47e06ad0100000000ffffffff0137aa0b000000000017a914050377baa6e8c5a07aed125d0ef262c6d5b67a038705483045022100d780139514f39ed943179e4638a519101bae875ec1220b226002bcbcb147830b0220273d1efb1514a77ee3dd4adee0e896b7e76be56c6d8e73470ae9bd91c91d700c01210344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f20ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e01015b63a82091d6a24697ed31932537ae598d3de3131e1fcd0641b9ac4be7afcb376386d71e8876a9149f4a0cf348b478336cb1d87ea4c8313a7ca3de1967029000b27576a91465252e57f727a27f32c77098e14d88d8dbec01816888ac00000000";

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
            txid: sha256d::Hash::from_hex(tx).unwrap(),
            vout,
        }
    }

    #[test]
    fn given_transaction_with_to_then_to_address_pattern_matches() {
        let tx = parse_raw_tx(WITNESS_TX);

        let pattern = TransactionPattern {
            to_address: Some("329XTScM6cJgu8VZvaqYWpfuxT1eQDSJkP".parse().unwrap()),
            from_outpoint: None,
            unlock_script: None,
        };

        let result = pattern.matches(&tx);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_a_witness_transaction_with_unlock_script_then_unlock_script_pattern_matches() {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "01",
        ]);

        let pattern = TransactionPattern {
            to_address: None,
            from_outpoint: None,
            unlock_script: Some(unlock_script),
        };

        let result = pattern.matches(&tx);
        assert_that(&result).is_true();
    }

    #[test]
    fn given_a_witness_transaction_with_different_unlock_script_then_unlock_script_pattern_wont_match(
    ) {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec!["102030405060708090", "00"]);

        let pattern = TransactionPattern {
            to_address: None,
            from_outpoint: None,
            unlock_script: Some(unlock_script),
        };

        let result = pattern.matches(&tx);
        assert_that(&result).is_false();
    }

    #[test]
    fn given_a_witness_transaction_with_unlock_script_then_spends_from_with_pattern_match() {
        let tx = parse_raw_tx(WITNESS_TX);
        let unlock_script = create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "01",
        ]);
        let outpoint = create_outpoint(
            "ad067ee417ee5518122374307d1fa494c67e30c75d38c7061d944b59e56fe024",
            1u32,
        );

        let pattern = TransactionPattern {
            to_address: None,
            from_outpoint: Some(outpoint),
            unlock_script: Some(unlock_script),
        };

        let result = pattern.matches(&tx);
        assert_that(&result).is_true();
    }
}
