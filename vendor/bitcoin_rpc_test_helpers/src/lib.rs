#![warn(unused_extern_crates, missing_debug_implementations)]
#![deny(unsafe_code)]

// Place for putting common queries needed in tests
extern crate bitcoin_rpc_client;
extern crate bitcoin_support;

use bitcoin_rpc_client::{
    BitcoinRpcApi, TransactionId, TransactionOutput, TxOutConfirmations, UnspentTransactionOutput,
};
use bitcoin_support::{Address, BitcoinQuantity, Network, Sha256dHash, ToP2wpkhAddress};

//TODO: All of this should be under #[cfg(test)]
pub trait RegtestHelperClient {
    fn find_utxo_at_tx_for_address(
        &self,
        txid: &TransactionId,
        address: &Address,
    ) -> Option<UnspentTransactionOutput>;
    fn find_vout_for_address(&self, txid: &TransactionId, address: &Address) -> TransactionOutput;

    fn enable_segwit(&self);
    fn create_p2wpkh_vout_at<D: ToP2wpkhAddress>(
        &self,
        dest: D,
        value: BitcoinQuantity,
    ) -> (Sha256dHash, TransactionOutput);
}

impl<Rpc: BitcoinRpcApi> RegtestHelperClient for Rpc {
    fn enable_segwit(&self) {
        self.generate(432).unwrap().unwrap();
    }

    fn find_utxo_at_tx_for_address(
        &self,
        txid: &TransactionId,
        address: &Address,
    ) -> Option<UnspentTransactionOutput> {
        let unspent = self
            .list_unspent(
                TxOutConfirmations::AtLeast(1),
                None,
                Some(vec![address.clone().into()]),
            ).unwrap()
            .unwrap();

        unspent.into_iter().find(|utxo| utxo.txid == *txid)
    }

    fn find_vout_for_address(&self, txid: &TransactionId, address: &Address) -> TransactionOutput {
        let raw_txn = self.get_raw_transaction_serialized(&txid).unwrap().unwrap();

        let decoded_txn = self
            .decode_rawtransaction(raw_txn.clone())
            .unwrap()
            .unwrap();

        decoded_txn
            .vout
            .iter()
            .find(|txout| txout.script_pub_key.hex == address.to_address().script_pubkey())
            .unwrap()
            .clone()
    }

    fn create_p2wpkh_vout_at<D: ToP2wpkhAddress>(
        &self,
        dest: D,
        value: BitcoinQuantity,
    ) -> (Sha256dHash, TransactionOutput) {
        let address = dest.to_p2wpkh_address(Network::Regtest);

        let txid = self
            .send_to_address(&address.clone().into(), value.bitcoin())
            .unwrap()
            .unwrap();

        self.generate(1).unwrap().unwrap();

        let vout = self.find_vout_for_address(&txid, &address);

        (txid.into(), vout)
    }
}
