// Place for putting common queries needed in tests
use BitcoinRpcApi;
use bitcoin::util::address::Address as BitcoinAddress;
use bitcoin_support::{BitcoinQuantity, Network, Sha256dHash, ToP2wpkhAddress};
use bitcoincore::TxOutConfirmations;
use types::{TransactionId, TransactionOutput, UnspentTransactionOutput};

pub trait RegtestHelperClient {
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
    fn create_p2wpkh_vout_at<D: ToP2wpkhAddress>(
        &self,
        dest: D,
        value: BitcoinQuantity,
    ) -> (Sha256dHash, TransactionOutput);
}

impl<Rpc: BitcoinRpcApi> RegtestHelperClient for Rpc {
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
            .find(|txout| txout.script_pub_key.hex == address.script_pubkey())
            .unwrap()
            .clone()
    }

    fn create_p2wpkh_vout_at<D: ToP2wpkhAddress>(
        &self,
        dest: D,
        value: BitcoinQuantity,
    ) -> (Sha256dHash, TransactionOutput) {
        let address = dest.to_p2wpkh_address(Network::BitcoinCoreRegtest);

        let txid = self.send_to_address(&address.clone().into(), value.bitcoin())
            .unwrap()
            .into_result()
            .unwrap();

        self.generate(1).unwrap();

        let vout = self.find_vout_for_address(&txid, &address);

        (txid.into(), vout)
    }
}
