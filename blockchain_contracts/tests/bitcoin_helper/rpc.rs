#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use rust_bitcoin::{
    hashes::sha256d::Hash as Sha256dHash, Address, Amount, Network, OutPoint, PublicKey, TxOut,
};

pub trait RegtestHelperClient {
    fn find_utxo_at_tx_for_address(&self, txid: &Sha256dHash, address: &Address) -> Option<TxOut>;
    fn find_vout_for_address(&self, txid: &Sha256dHash, address: &Address) -> OutPoint;
    fn mine_bitcoins(&self);
    fn create_p2wpkh_vout_at(
        &self,
        dest: rust_bitcoin::secp256k1::PublicKey,
        value: Amount,
    ) -> (Sha256dHash, OutPoint);
}

impl<Rpc: bitcoincore_rpc::RpcApi> RegtestHelperClient for Rpc {
    fn find_utxo_at_tx_for_address(&self, txid: &Sha256dHash, address: &Address) -> Option<TxOut> {
        let address = address.clone();
        let unspent = self
            .list_unspent(Some(1), None, Some(&[address]), None, None)
            .unwrap();

        #[allow(clippy::cast_sign_loss)] // it is just for the tests
        unspent
            .into_iter()
            .find(|utxo| utxo.txid == *txid)
            .map(|result| TxOut {
                value: result.amount.as_sat(),
                script_pubkey: result.script_pub_key,
            })
    }

    fn find_vout_for_address(&self, txid: &Sha256dHash, address: &Address) -> OutPoint {
        let tx = self.get_raw_transaction(&txid, None).unwrap();

        #[allow(clippy::cast_possible_truncation)]
        // there will never be tx with more than u32::MAX outputs
        tx.output
            .iter()
            .enumerate()
            .find_map(|(vout, txout)| {
                if txout.script_pubkey == address.script_pubkey() {
                    Some(OutPoint {
                        txid: *txid,
                        vout: vout as u32,
                    })
                } else {
                    None
                }
            })
            .unwrap()
    }

    fn mine_bitcoins(&self) {
        self.generate(101, None).unwrap();
    }

    fn create_p2wpkh_vout_at(
        &self,
        public_key: rust_bitcoin::secp256k1::PublicKey,
        amount: Amount,
    ) -> (Sha256dHash, OutPoint) {
        let address = Address::p2wpkh(
            &PublicKey {
                compressed: true,
                key: public_key,
            },
            Network::Regtest,
        );

        let txid = self
            .send_to_address(&address.clone(), amount, None, None, None, None, None, None)
            .unwrap();

        self.generate(1, None).unwrap();

        let vout = self.find_vout_for_address(&txid, &address);

        (txid, vout)
    }
}
