// Place for putting common queries needed in tests
use BitcoinRpcApi;
use bitcoin::util::address::Address as BitcoinAddress;
use bitcoincore::TxOutConfirmations;
use std_hex;
use types::{TransactionId, TransactionOutput, UnspentTransactionOutput};

pub trait TestUtility {
    fn find_utxo_at_tx_for_address(
        &self,
        txid: &TransactionId,
        address: &BitcoinAddress,
    ) -> Option<UnspentTransactionOutput>;
    fn find_vout_for_address(
        &self,
        txid: &TransactionId,
        address: &BitcoinAddress,
    ) -> TransactionOutput;

    fn enable_segwit(&self);
}

impl<Rpc: BitcoinRpcApi> TestUtility for Rpc {
    fn enable_segwit(&self) {
        self.generate(432).unwrap();
    }

    fn find_utxo_at_tx_for_address(
        &self,
        txid: &TransactionId,
        address: &BitcoinAddress,
    ) -> Option<UnspentTransactionOutput> {
        let unspent = self.list_unspent(
            TxOutConfirmations::AtLeast(1),
            None,
            Some(vec![address.clone().into()]),
        ).unwrap()
            .into_result()
            .unwrap();

        unspent.into_iter().find(|utxo| utxo.txid == *txid)
    }

    fn find_vout_for_address(
        &self,
        txid: &TransactionId,
        address: &BitcoinAddress,
    ) -> TransactionOutput {
        let raw_txn = self.get_raw_transaction_serialized(&txid)
            .unwrap()
            .into_result()
            .unwrap();

        let decoded_txn = self.decode_rawtransaction(raw_txn.clone())
            .unwrap()
            .into_result()
            .unwrap();

        decoded_txn
            .vout
            .iter()
            .find(|txout| {
                std_hex::decode(&txout.script_pub_key.hex).unwrap()
                    == address.script_pubkey().into_vec()
            })
            .unwrap()
            .clone()
    }
}
