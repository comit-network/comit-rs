#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use bitcoin_support::{
    Address, Amount, IntoP2wpkhAddress, Network, OutPoint, Sha256dHash, TransactionId, TxOut,
};

pub trait RegtestHelperClient {
    fn find_utxo_at_tx_for_address(&self, txid: &TransactionId, address: &Address)
        -> Option<TxOut>;
    fn find_vout_for_address(&self, txid: &TransactionId, address: &Address) -> OutPoint;
    fn mine_bitcoins(&self);
    fn create_p2wpkh_vout_at<D: IntoP2wpkhAddress>(
        &self,
        dest: D,
        value: Amount,
    ) -> (Sha256dHash, OutPoint);
}

impl<Rpc: bitcoincore_rpc::RpcApi> RegtestHelperClient for Rpc {
    fn find_utxo_at_tx_for_address(
        &self,
        txid: &TransactionId,
        address: &Address,
    ) -> Option<TxOut> {
        // So that comit-rs can use latest rust-bitcoin even if bitcoincore-rpc does not
        let address = address.to_string().parse().unwrap();
        let unspent = self
            .list_unspent(Some(1), None, Some(&[address]), None, None)
            .unwrap();

        #[allow(clippy::cast_sign_loss)] // it is just for the tests
        unspent
            .into_iter()
            .find(|utxo| utxo.txid == *txid)
            .map(|result| {
                let script_pubkey = result.script_pub_key.to_bytes().into();
                TxOut {
                    value: result.amount.as_sat(),
                    script_pubkey,
                }
            })
    }

    fn find_vout_for_address(&self, txid: &TransactionId, address: &Address) -> OutPoint {
        let tx = self.get_raw_transaction(&txid, None).unwrap();
        let script_pubkey = address.script_pubkey().to_bytes().into();
        #[allow(clippy::cast_possible_truncation)]
        // there will never be tx with more than u32::MAX outputs
        tx.output
            .iter()
            .enumerate()
            .find_map(|(vout, txout)| {
                if txout.script_pubkey == script_pubkey {
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

    fn create_p2wpkh_vout_at<D: IntoP2wpkhAddress>(
        &self,
        dest: D,
        amount: Amount,
    ) -> (Sha256dHash, OutPoint) {
        let address = dest.into_p2wpkh_address(Network::Regtest);

        // So that comit-rs can use latest rust-bitcoin even if bitcoincore-rpc does not
        let address_rpc = address.clone().to_string().parse().unwrap();
        let amount_rpc = format!("{} sat", amount.as_sat()).parse().unwrap();

        let txid = self
            .send_to_address(&address_rpc, amount_rpc, None, None, None, None, None, None)
            .unwrap();

        self.generate(1, None).unwrap();

        let vout = self.find_vout_for_address(&txid, &address);

        (txid, vout)
    }
}
