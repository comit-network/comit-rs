use crate::script::Instruction::{Error, Op, PushBytes};
use bitcoin::{
    blockdata::transaction::Transaction, util::address::Address as BitcoinAddress, OutPoint, TxIn,
    TxOut,
};

pub trait SpendsTo {
    fn spends_to(&self, address: &BitcoinAddress) -> bool;
}

pub trait SpendsFrom {
    fn spends_from(&self, outpoint: &OutPoint) -> bool;
}

pub trait SpendsFromWith {
    fn spends_from_with(&self, outpoint: &OutPoint, script: &[Vec<u8>]) -> bool;
}

pub trait SpendsWith {
    fn spends_with(&self, script: &[Vec<u8>]) -> bool;
}

impl SpendsTo for Transaction {
    fn spends_to(&self, address: &BitcoinAddress) -> bool {
        let address_script_pubkey = address.script_pubkey();

        self.output
            .iter()
            .map(|out| &out.script_pubkey)
            .any(|script_pub_key| script_pub_key == &address_script_pubkey)
    }
}

impl SpendsFrom for Transaction {
    fn spends_from(&self, outpoint: &OutPoint) -> bool {
        self.input
            .iter()
            .map(|input| &input.previous_output)
            .any(|previous_outpoint| previous_outpoint == outpoint)
    }
}

impl SpendsFromWith for Transaction {
    fn spends_from_with(&self, outpoint: &OutPoint, unlock_script: &[Vec<u8>]) -> bool {
        self.input
            .iter()
            .filter(|previous_outpoint| &previous_outpoint.previous_output == outpoint)
            .any(|txin| any_unlock_script_matches(txin, unlock_script))
    }
}

impl SpendsWith for Transaction {
    fn spends_with(&self, unlock_script: &[Vec<u8>]) -> bool {
        self.input
            .iter()
            .any(|txin| any_unlock_script_matches(txin, unlock_script))
    }
}

fn any_unlock_script_matches(txin: &TxIn, unlock_script: &[Vec<u8>]) -> bool {
    unlock_script.iter().all(|item| {
        txin.witness.contains(item)
            || unlock_script.iter().all(|item| {
                txin.script_sig
                    .iter(true)
                    .any(|instruction| match instruction {
                        PushBytes(data) => (item as &[u8]) == data,
                        Op(_) => false,
                        Error(_) => false,
                    })
            })
    })
}

pub trait FindOutput {
    fn find_output(&self, to_address: &BitcoinAddress) -> Option<(u32, &TxOut)>;
}

#[allow(clippy::cast_possible_truncation)]
impl FindOutput for Transaction {
    fn find_output(&self, to_address: &BitcoinAddress) -> Option<(u32, &TxOut)> {
        let to_address_script_pubkey = to_address.script_pubkey();

        self.output
            .iter()
            .enumerate()
            .map(|(index, txout)| (index as u32, txout))
            .find(|(_, txout)| txout.script_pubkey == to_address_script_pubkey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sha256dHash;
    use bitcoin::{blockdata::transaction::TxOut, consensus::deserialize};
    use bitcoin_hashes::hex::FromHex;
    use spectral::prelude::*;

    const WITNESS_TX: & str = "0200000000010124e06fe5594b941d06c7385dc7307ec694a41f7d307423121855ee17e47e06ad0100000000ffffffff0137aa0b000000000017a914050377baa6e8c5a07aed125d0ef262c6d5b67a038705483045022100d780139514f39ed943179e4638a519101bae875ec1220b226002bcbcb147830b0220273d1efb1514a77ee3dd4adee0e896b7e76be56c6d8e73470ae9bd91c91d700c01210344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f20ec9e9fb3c669b2354ea026ab3da82968a2e7ab9398d5cbed4e78e47246f2423e01015b63a82091d6a24697ed31932537ae598d3de3131e1fcd0641b9ac4be7afcb376386d71e8876a9149f4a0cf348b478336cb1d87ea4c8313a7ca3de1967029000b27576a91465252e57f727a27f32c77098e14d88d8dbec01816888ac00000000";
    const STANDRD_TX: & str = "01000000013dcd7d87904c9cb7f4b79f36b5a03f96e2e729284c09856238d5353e1182b00200000000fd5e0100483045022100deeb1f13b5927b5e32d877f3c42a4b028e2e0ce5010fdb4e7f7b5e2921c1dcd2022068631cb285e8c1be9f061d2968a18c3163b780656f30a049effee640e80d9bff01483045022100ee80e164622c64507d243bd949217d666d8b16486e153ac6a1f8e04c351b71a502203691bef46236ca2b4f5e60a82a853a33d6712d6a1e7bf9a65e575aeb7328db8c014cc9524104a882d414e478039cd5b52a92ffb13dd5e6bd4515497439dffd691a0f12af9575fa349b5694ed3155b136f09e63975a1700c9f4d4df849323dac06cf3bd6458cd41046ce31db9bdd543e72fe3039a1f1c047dab87037c36a669ff90e28da1848f640de68c2fe913d363a51154a0c62d7adea1b822d05035077418267b1a1379790187410411ffd36c70776538d079fbae117dc38effafb33304af83ce4894589747aee1ef992f63280567f52f5ba870678b4ab4ff6c8ea600bd217870a8b4f1f09f3a8e8353aeffffffff0130d90000000000001976a914569076ba39fc4ff6a2291d9ea9196d8c08f9c7ab88ac00000000";

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

    fn create_valid_p2wsh_unlock_script() -> Vec<Vec<u8>> {
        create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "01",
        ])
    }

    fn create_invalid_p2wsh_unlock_script() -> Vec<Vec<u8>> {
        create_unlock_script_stack(vec![
            "0344f8f459494f74ebb87464de9b74cdba3709692df4661159857988966f94262f",
            "00",
        ])
    }

    fn create_valid_p2sh_unlock_script() -> Vec<Vec<u8>> {
        create_unlock_script_stack(vec!["3045022100deeb1f13b5927b5e32d877f3c42a4b028e2e0ce5010fdb4e7f7b5e2921c1dcd2022068631cb285e8c1be9f061d2968a18c3163b780656f30a049effee640e80d9bff01",
                                        "3045022100ee80e164622c64507d243bd949217d666d8b16486e153ac6a1f8e04c351b71a502203691bef46236ca2b4f5e60a82a853a33d6712d6a1e7bf9a65e575aeb7328db8c01"])
    }

    fn create_invalid_p2sh_unlock_script() -> Vec<Vec<u8>> {
        create_unlock_script_stack(vec!["3045022100deeb1f13b5927b5e32d877f3c42a4b028e2e0ce5010fdb4e7f7b5e2921c1dcd2022068631cb285e8c1be9f061d2968a18c3163b780656f30a049effee640e80d9bff01",
                                        "3045022100deeb1f13b5927b5e32d877f3c42a4b028e2e0ce5010fdb4e7f7b5e2921c1dcd2022068631cb285e8c1be9f061d2968a18c3163b780656f30a049effee640e80d9bff01",
                                        "0101"])
    }

    fn create_valid_p2sh_outpoint() -> OutPoint {
        create_outpoint(
            "02b082113e35d5386285094c2829e7e2963fa0b5369fb7f4b79c4c90877dcd3d",
            0u32,
        )
    }

    fn create_invalid_p2sh_outpoint() -> OutPoint {
        create_outpoint(
            "ad067ee417ee5518122374307d1fa494c67e30c75d38c7061d944b59e56fe024",
            1u32,
        )
    }

    #[test]
    fn tx_with_txout_should_return_true() {
        let address: BitcoinAddress = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap();
        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: Vec::new(),
            output: vec![TxOut {
                value: 0,
                script_pubkey: address.script_pubkey(),
            }],
        };

        assert_that(&tx.spends_to(&address)).is_true();
    }

    #[test]
    fn tx_spending_to_other_address_returns_false() {
        let address1: BitcoinAddress = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap();
        let address2: BitcoinAddress = "bc1qu5t5yrh75zca6msxzszx5mm0egu2vepu09lwqh"
            .parse()
            .unwrap();

        let tx = Transaction {
            version: 1,
            lock_time: 0,
            input: Vec::new(),
            output: vec![TxOut {
                value: 0,
                script_pubkey: address1.script_pubkey(),
            }],
        };

        assert_that(&tx.spends_to(&address2)).is_false();
    }

    #[test]
    fn a_witness_tx_with_unlock_script_then_unlock_script_contains_matches() {
        let tx = parse_raw_tx(WITNESS_TX);

        let unlock_script = create_valid_p2wsh_unlock_script();

        assert_that(&tx.spends_with(&unlock_script)).is_true();
    }

    #[test]
    fn a_witness_tx_with_different_unlock_script_then_unlock_script_contains_wont_match() {
        let tx = parse_raw_tx(WITNESS_TX);

        let unlock_script = create_invalid_p2wsh_unlock_script();

        assert_that(&tx.spends_with(&unlock_script)).is_false();
    }

    #[test]
    fn a_p2sh_tx_with_unlock_script_then_unlock_script_matches() {
        let tx = parse_raw_tx(STANDRD_TX);

        let unlock_script = create_valid_p2sh_unlock_script();

        assert_that(&tx.spends_with(&unlock_script)).is_true();
    }

    #[test]
    fn a_p2sh_tx_with_additional_unlock_script_then_unlock_script_wont_match() {
        let tx = parse_raw_tx(STANDRD_TX);

        let unlock_script = create_invalid_p2sh_unlock_script();

        assert_that(&tx.spends_with(&unlock_script)).is_false();
    }

    #[test]
    fn a_tx_with_spends_from_outpoint_matches() {
        let tx = parse_raw_tx(STANDRD_TX);

        let outpoint = create_valid_p2sh_outpoint();

        assert_that(&tx.spends_from(&outpoint)).is_true();
    }

    #[test]
    fn a_tx_which_spends_from_different_outpoint_does_not_match() {
        let tx = parse_raw_tx(STANDRD_TX);

        let outpoint = create_invalid_p2sh_outpoint();

        assert_that(&tx.spends_from(&outpoint)).is_false();
    }

    #[test]
    fn a_tx_spending_from_tx_with_specific_script_then_spend_from_with_matches() {
        let tx = parse_raw_tx(STANDRD_TX);

        let unlock_script = create_valid_p2sh_unlock_script();
        let outpoint = create_valid_p2sh_outpoint();

        assert_that(&tx.spends_from_with(&outpoint, &unlock_script)).is_true();
    }

    #[test]
    fn a_tx_spending_from_tx_with_different_script_then_spends_from_with_does_not_match() {
        let tx = parse_raw_tx(STANDRD_TX);

        let unlock_script = create_invalid_p2sh_unlock_script();
        let outpoint = create_valid_p2sh_outpoint();

        assert_that(&tx.spends_from_with(&outpoint, &unlock_script)).is_false();
    }
}
