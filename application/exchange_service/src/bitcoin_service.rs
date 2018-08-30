use bitcoin_fee_service::{self, BitcoinFeeService};
use bitcoin_htlc::bitcoin_htlc;
use bitcoin_rpc_client::{self, TransactionId};
use bitcoin_support::{self, PubkeyHash};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    secret::{Secret, SecretHash},
};
use reqwest;
use std::sync::Arc;
use swaps::{
    common::TradeId,
    events::{OfferCreated, OrderTaken, TradeFunded},
};

#[derive(Debug)]
pub enum Error {
    Unlocking,
    NodeConnection,
    Internal,
}

impl From<reqwest::Error> for Error {
    fn from(_error: reqwest::Error) -> Self {
        Error::NodeConnection
    }
}

impl From<bitcoin_rpc_client::RpcError> for Error {
    fn from(_error: bitcoin_rpc_client::RpcError) -> Self {
        Error::NodeConnection
    }
}

impl From<bitcoin_htlc::UnlockingError> for Error {
    fn from(_error: bitcoin_htlc::UnlockingError) -> Self {
        Error::Unlocking
    }
}

impl From<bitcoin_fee_service::Error> for Error {
    fn from(_error: bitcoin_fee_service::Error) -> Self {
        Error::Internal
    }
}

pub struct BitcoinService {
    client: Arc<bitcoin_rpc_client::BitcoinRpcApi>,
    fee_service: Arc<BitcoinFeeService>,
    network: bitcoin_support::Network,
    btc_exchange_redeem_address: bitcoin_support::Address,
}

pub trait LedgerHtlcService<B: Ledger>: Send + Sync {
    fn deploy_htlc(
        &self,
        refund_address: B::Address,
        success_address: B::Address,
        time_lock: B::Time,
        amount: B::Quantity,
        secret: SecretHash,
    ) -> Result<B::TxId, Error>;
}

impl LedgerHtlcService<Bitcoin> for BitcoinService {
    fn deploy_htlc(
        &self,
        refund_address: <Bitcoin as Ledger>::Address,
        success_address: <Bitcoin as Ledger>::Address,
        time_lock: <Bitcoin as Ledger>::Time,
        amount: <Bitcoin as Ledger>::Quantity,
        secret: SecretHash,
    ) -> Result<<Bitcoin as Ledger>::TxId, Error> {
        let htlc =
            bitcoin_htlc::Htlc::new(success_address, refund_address, secret, time_lock.into());

        let htlc_address = htlc.compute_address(self.network);

        let tx_id = self
            .client
            .send_to_address(&htlc_address.clone().into(), amount.bitcoin())??;

        Ok(tx_id)
    }
}

impl BitcoinService {
    pub fn new(
        client: Arc<bitcoin_rpc_client::BitcoinRpcApi>,
        network: bitcoin_support::Network,
        fee_service: Arc<BitcoinFeeService>,
        btc_exchange_redeem_address: bitcoin_support::Address,
    ) -> Self {
        BitcoinService {
            client,
            fee_service,
            network,
            btc_exchange_redeem_address,
        }
    }

    pub fn deploy_htlc(
        &self,
        order_taken_event: OrderTaken<Bitcoin, Ethereum>,
        offer_created_event: OfferCreated<Bitcoin, Ethereum>,
    ) -> Result<TransactionId, Error> {
        let exchange_refund_address = order_taken_event.exchange_refund_address;
        let exchange_refund_address_pub_key: PubkeyHash = exchange_refund_address.into();
        let client_success_address = order_taken_event.client_success_address;
        let client_success_address_hash: PubkeyHash = client_success_address.into();

        let amount = offer_created_event.buy_amount;

        let htlc = bitcoin_htlc::Htlc::new(
            client_success_address_hash,
            exchange_refund_address_pub_key,
            order_taken_event.contract_secret_lock.clone(),
            order_taken_event.exchange_contract_time_lock.into(),
        );

        let htlc_address = htlc.compute_address(self.network);

        let tx_id = self
            .client
            .send_to_address(&htlc_address.clone().into(), amount.bitcoin())??;

        Ok(tx_id)
    }

    pub fn redeem_htlc(
        &self,
        secret: Secret,
        trade_id: TradeId,
        order_taken_event: OrderTaken<Ethereum, Bitcoin>,
        offer_created_event: OfferCreated<Ethereum, Bitcoin>,
        trade_funded_event: TradeFunded<Ethereum, Bitcoin>,
    ) -> Result<TransactionId, Error> {
        let exchange_success_address = order_taken_event.exchange_success_address;
        let exchange_success_pubkey_hash: PubkeyHash = exchange_success_address.into();
        let exchange_success_keypair = order_taken_event.exchange_success_keypair;

        let client_refund_pubkey_hash: PubkeyHash = order_taken_event.client_refund_address.into();
        let htlc_tx_id = trade_funded_event.htlc_identifier.transaction_id;
        let vout = trade_funded_event.htlc_identifier.vout;

        let htlc = bitcoin_htlc::Htlc::new(
            exchange_success_pubkey_hash,
            client_refund_pubkey_hash,
            order_taken_event.contract_secret_lock.clone(),
            order_taken_event.client_contract_time_lock.clone().into(),
        );

        htlc.can_be_unlocked_with(&secret, &exchange_success_keypair)?;

        let unlocking_parameters =
            htlc.unlock_with_secret(exchange_success_keypair.clone(), secret);

        let primed_txn = PrimedTransaction {
            inputs: vec![PrimedInput::new(
                htlc_tx_id.clone().into(),
                vout,
                offer_created_event.sell_amount,
                unlocking_parameters,
            )],
            output_address: self.btc_exchange_redeem_address.clone(),
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
}
