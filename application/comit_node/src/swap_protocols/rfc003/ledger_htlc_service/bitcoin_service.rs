use bitcoin_fee_service::{self, BitcoinFeeService};
use bitcoin_htlc::bitcoin_htlc;
use bitcoin_rpc_client;
use bitcoin_support::{self, PubkeyHash, Script, Transaction, TxOut};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use common_types::secret::{Secret, SecretHash};
use reqwest;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swap_protocols::{
    ledger::{bitcoin::Bitcoin, Ledger},
    rfc003::ledger_htlc_service::{self, LedgerHtlcService},
};
use swaps::common::TradeId;

impl From<reqwest::Error> for ledger_htlc_service::Error {
    fn from(error: reqwest::Error) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::NodeConnection
    }
}

impl From<bitcoin_rpc_client::RpcError> for ledger_htlc_service::Error {
    fn from(error: bitcoin_rpc_client::RpcError) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::NodeConnection
    }
}

impl From<bitcoin_htlc::UnlockingError> for ledger_htlc_service::Error {
    fn from(error: bitcoin_htlc::UnlockingError) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::Unlocking
    }
}

impl From<bitcoin_fee_service::Error> for ledger_htlc_service::Error {
    fn from(error: bitcoin_fee_service::Error) -> Self {
        error!("{:?}", error);
        ledger_htlc_service::Error::Internal
    }
}

#[derive(DebugStub)]
pub struct BitcoinService {
    #[debug_stub = "BitcoinRpcClient"]
    client: Arc<bitcoin_rpc_client::BitcoinRpcApi>,
    #[debug_stub = "FeeService"]
    fee_service: Arc<BitcoinFeeService>,
    network: bitcoin_support::Network,
    btc_bob_redeem_address: bitcoin_support::Address,
}

use bitcoin_support::{Address, BitcoinQuantity, Blocks};

// TODO: Maybe interesting to refactor and have the bitcoin service generate the
// transient/redeem keypairs transparently (ie, receiving the keystore) see #296
#[derive(Debug)]
pub struct BitcoinHtlcParams {
    pub refund_address: Address,
    pub success_address: Address,
    pub time_lock: Blocks,
    pub amount: BitcoinQuantity,
    pub secret_hash: SecretHash,
}

impl LedgerHtlcService<Bitcoin, BitcoinHtlcParams> for BitcoinService {
    fn deploy_htlc(
        &self,
        htlc_params: BitcoinHtlcParams,
    ) -> Result<<Bitcoin as Ledger>::TxId, ledger_htlc_service::Error> {
        let htlc = bitcoin_htlc::Htlc::new(
            htlc_params.success_address,
            htlc_params.refund_address,
            htlc_params.secret_hash,
            htlc_params.time_lock.into(),
        );

        let htlc_address = htlc.compute_address(self.network);

        let tx_id = self
            .client
            .send_to_address(&htlc_address.clone().into(), htlc_params.amount.bitcoin())??;

        Ok(tx_id)
    }

    fn redeem_htlc(
        &self,
        secret: Secret,
        trade_id: TradeId,
        bob_success_address: <Bitcoin as Ledger>::Address,
        bob_success_keypair: KeyPair,
        alice_refund_address: <Bitcoin as Ledger>::Address,
        htlc_identifier: <Bitcoin as Ledger>::HtlcId,
        sell_amount: <Bitcoin as Ledger>::Quantity,
        lock_time: <Bitcoin as Ledger>::LockDuration,
    ) -> Result<<Bitcoin as Ledger>::TxId, ledger_htlc_service::Error> {
        let bob_success_pubkey_hash: PubkeyHash = bob_success_address.into();

        let alice_refund_pubkey_hash: PubkeyHash = alice_refund_address.into();
        let htlc_tx_id = htlc_identifier.transaction_id;
        let vout = htlc_identifier.vout;

        let htlc = bitcoin_htlc::Htlc::new(
            bob_success_pubkey_hash,
            alice_refund_pubkey_hash,
            secret.hash(),
            lock_time.clone().into(),
        );

        htlc.can_be_unlocked_with(secret, bob_success_keypair)?;

        let unlocking_parameters = htlc.unlock_with_secret(bob_success_keypair, &secret);

        let primed_txn = PrimedTransaction {
            inputs: vec![PrimedInput::new(
                htlc_tx_id.clone().into(),
                vout,
                sell_amount,
                unlocking_parameters,
            )],
            output_address: self.btc_bob_redeem_address.clone(),
            locktime: 0,
        };

        let total_input_value = primed_txn.total_input_value();

        let rate = self.fee_service.get_recommended_fee()?;
        let redeem_tx = primed_txn.sign_with_rate(rate);
        debug!(
            "Redeem {} (input: {}, vout: {}) to {} (output: {})",
            htlc_tx_id,
            total_input_value,
            vout,
            redeem_tx.txid(),
            redeem_tx.output[0].value
        );

        let rpc_transaction = bitcoin_rpc_client::SerializedRawTransaction::from(redeem_tx);
        info!(
            "Attempting to redeem HTLC with txid {} for {}",
            htlc_tx_id, trade_id
        );

        let redeem_txid = self.client.send_raw_transaction(rpc_transaction)??;

        info!(
            "HTLC for {} successfully redeemed with {}",
            trade_id, redeem_txid
        );

        Ok(redeem_txid)
    }

    fn create_query_to_watch_redeeming(
        &self,
        _htlc_funding_tx_id: <Bitcoin as Ledger>::TxId,
    ) -> Result<<Bitcoin as Ledger>::QueryForLedgerQueryService, ledger_htlc_service::Error> {
        unimplemented!()
    }

    fn check_and_extract_secret(
        &self,
        _create_htlc_tx_id: <Bitcoin as Ledger>::TxId,
        _redeem_htlc_tx_id: <Bitcoin as Ledger>::TxId,
    ) -> Result<Secret, ledger_htlc_service::Error> {
        unimplemented!()
    }
}

impl BitcoinService {
    pub fn new(
        client: Arc<bitcoin_rpc_client::BitcoinRpcApi>,
        network: bitcoin_support::Network,
        fee_service: Arc<BitcoinFeeService>,
        btc_bob_redeem_address: bitcoin_support::Address,
    ) -> Self {
        BitcoinService {
            client,
            fee_service,
            network,
            btc_bob_redeem_address,
        }
    }

    pub fn get_vout_matching(
        &self,
        txid: &bitcoin_rpc_client::TransactionId,
        script: &Script,
    ) -> Result<Option<(usize, TxOut)>, ledger_htlc_service::Error> {
        let transaction: Transaction = self.client.get_raw_transaction_serialized(&txid)??.into();
        Ok(transaction
            .output
            .into_iter()
            .enumerate()
            .find(|(_, txout)| &txout.script_pubkey == script))
    }
}
