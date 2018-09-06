use bitcoin::{blockdata::transaction::Transaction, util::address::Address as BitcoinAddress};

pub trait SpendsTo {
    fn spends_to(&self, address: &BitcoinAddress) -> bool;
}

impl SpendsTo for Transaction {
    fn spends_to(&self, address: &BitcoinAddress) -> bool {
        let address_script_pubkey = address.script_pubkey();

        self.output
            .iter()
            .map(|out| &out.script_pubkey)
            .find(|script_pub_key| *script_pub_key == &address_script_pubkey)
            .is_some()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use bitcoin::blockdata::transaction::TxOut;
    use spectral::prelude::*;

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

}
