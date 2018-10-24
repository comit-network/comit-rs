use bitcoin_fee_service::{self, BitcoinFeeService};
use bitcoin_rpc_client::{self, *};
use bitcoin_support::{
    self, Address, BitcoinQuantity, Blocks, PubkeyHash, Script, Transaction, TxOut,
};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use ledger_query_service::BitcoinQuery;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swap_protocols::{
    ledger::{bitcoin::Bitcoin, Ledger},
    rfc003::{
        bitcoin,
        ledger_htlc_service::{self, LedgerHtlcService},
        Secret, SecretHash,
    },
};
use swaps::common::TradeId;

impl From<bitcoin_rpc_client::ClientError> for ledger_htlc_service::Error {
    fn from(error: bitcoin_rpc_client::ClientError) -> Self {
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

impl From<bitcoin::UnlockingError> for ledger_htlc_service::Error {
    fn from(error: bitcoin::UnlockingError) -> Self {
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
    client: Arc<BitcoinRpcApi>,
    #[debug_stub = "FeeService"]
    fee_service: Arc<BitcoinFeeService>,
    network: bitcoin_support::Network,
    btc_bob_redeem_address: bitcoin_support::Address,
}

// TODO: Maybe interesting to refactor and have the bitcoin service generate the
// transient/redeem keypairs transparently (ie, receiving the keystore) see #296
#[derive(Clone, Debug)]
pub struct BitcoinHtlcFundingParams {
    pub refund_pubkey_hash: PubkeyHash,
    pub success_pubkey_hash: PubkeyHash,
    pub time_lock: Blocks,
    pub amount: BitcoinQuantity,
    pub secret_hash: SecretHash,
}

#[derive(Clone, Debug)]
pub struct BitcoinHtlcRedeemParams {
    pub htlc_identifier: <Bitcoin as Ledger>::HtlcId,
    pub success_address: Address,
    pub refund_address: Address,
    pub amount: BitcoinQuantity,
    pub time_lock: Blocks,
    pub keypair: KeyPair,
    pub secret: Secret,
}

impl LedgerHtlcService<Bitcoin, BitcoinHtlcFundingParams, BitcoinHtlcRedeemParams, BitcoinQuery>
    for BitcoinService
{
    fn fund_htlc(
        &self,
        htlc_funding_params: BitcoinHtlcFundingParams,
    ) -> Result<<Bitcoin as Ledger>::TxId, ledger_htlc_service::Error> {
        let htlc = bitcoin::Htlc::new(
            htlc_funding_params.success_pubkey_hash,
            htlc_funding_params.refund_pubkey_hash,
            htlc_funding_params.secret_hash,
            htlc_funding_params.time_lock.into(),
        );

        let htlc_address = htlc.compute_address(self.network);

        let tx_id = self
            .client
            .send_to_address(&htlc_address.into(), htlc_funding_params.amount.bitcoin())??;

        Ok(tx_id)
    }

    fn redeem_htlc(
        &self,
        trade_id: TradeId,
        htlc_redeem_params: BitcoinHtlcRedeemParams,
    ) -> Result<<Bitcoin as Ledger>::TxId, ledger_htlc_service::Error> {
        let bob_success_address = htlc_redeem_params.success_address;
        let alice_refund_address = htlc_redeem_params.refund_address;
        let sell_amount = htlc_redeem_params.amount;
        let lock_time = htlc_redeem_params.time_lock;
        let bob_success_keypair = htlc_redeem_params.keypair;
        let htlc_identifier = htlc_redeem_params.htlc_identifier;
        let secret = htlc_redeem_params.secret;

        let bob_success_pubkey_hash: PubkeyHash = bob_success_address.into();

        let alice_refund_pubkey_hash: PubkeyHash = alice_refund_address.into();

        let htlc = bitcoin::Htlc::new(
            bob_success_pubkey_hash,
            alice_refund_pubkey_hash,
            secret.hash(),
            lock_time.into(),
        );

        htlc.can_be_unlocked_with(secret, bob_success_keypair)?;

        let unlocking_parameters = htlc.unlock_with_secret(bob_success_keypair, &secret);

        let primed_txn = PrimedTransaction {
            inputs: vec![PrimedInput::new(
                htlc_identifier,
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
            "Redeem HTLC at {:?} with {} to {} (output: {})",
            htlc_identifier,
            total_input_value,
            redeem_tx.txid(),
            redeem_tx.output[0].value
        );

        let rpc_transaction = rpc::SerializedRawTransaction::from(redeem_tx);
        info!(
            "Attempting to redeem HTLC with {:?} for {}",
            htlc_identifier, trade_id
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
    ) -> Result<BitcoinQuery, ledger_htlc_service::Error> {
        unimplemented!()
    }

    fn create_query_to_watch_funding(&self, htlc_params: BitcoinHtlcFundingParams) -> BitcoinQuery {
        let htlc = bitcoin::Htlc::new(
            htlc_params.success_pubkey_hash,
            htlc_params.refund_pubkey_hash,
            htlc_params.secret_hash,
            htlc_params.time_lock.into(),
        );

        let htlc_address = htlc.compute_address(self.network);

        BitcoinQuery::Transaction {
            to_address: Some(htlc_address),
        }
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
        client: Arc<BitcoinRpcApi>,
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
        txid: &TransactionId,
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
