#![warn(unused_extern_crates, missing_debug_implementations, rust_2018_idioms)]
#![forbid(unsafe_code)]

use bitcoin::{hashes::sha256d, Address, OutPoint, TxOut};
use std::convert::TryFrom;

pub trait RegtestHelperClient {
    fn find_utxo_at_tx_for_address(&self, txid: &sha256d::Hash, address: &Address)
        -> Option<TxOut>;
    fn find_vout_for_address(&self, txid: &sha256d::Hash, address: &Address) -> OutPoint;
}

impl<Rpc: bitcoincore_rpc::RpcApi> RegtestHelperClient for Rpc {
    fn find_utxo_at_tx_for_address(
        &self,
        txid: &sha256d::Hash,
        address: &Address,
    ) -> Option<TxOut> {
        let address = address.clone();
        let unspent = self
            .list_unspent(Some(1), None, Some(&[address]), None, None)
            .unwrap();

        #[allow(clippy::cast_sign_loss)] // it is just for the tests
        unspent
            .into_iter()
            .find(|utxo| utxo.txid == *txid)
            .map(|result| {
                let value = u64::try_from(result.amount.as_sat()).unwrap();
                TxOut {
                    value,
                    script_pubkey: result.script_pub_key,
                }
            })
    }

    fn find_vout_for_address(&self, txid: &sha256d::Hash, address: &Address) -> OutPoint {
        let tx = self.get_raw_transaction(&txid, None).unwrap();

        tx.output
            .iter()
            .enumerate()
            .find_map(|(vout, txout)| {
                let vout = u32::try_from(vout).unwrap();
                if txout.script_pubkey == address.script_pubkey() {
                    Some(OutPoint { txid: *txid, vout })
                } else {
                    None
                }
            })
            .unwrap()
    }
}
