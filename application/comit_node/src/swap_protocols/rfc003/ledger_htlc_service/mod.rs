pub use self::{bitcoin_service::*, ethereum_service::*};
use common_types::secret::Secret;
use secp256k1_support::KeyPair;
use swap_protocols::ledger::Ledger;
use swaps::common::TradeId;

mod bitcoin_service;
mod ethereum_service;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Cannot unlock HTLC")]
    Unlocking,
    #[fail(display = "Node connection issue")]
    NodeConnection,
    #[fail(display = "Internal Error")]
    Internal,
    #[fail(display = "Provided transaction not found on the blockchain")]
    TransactionNotFound,
    #[fail(display = "The redeem transaction is malformed or invalid")]
    InvalidRedeemTransaction,
}

pub trait LedgerHtlcService<L: Ledger, H, Q>: Send + Sync {
    fn deploy_htlc(&self, htlc_params: H) -> Result<L::TxId, Error>;

    #[allow(clippy::too_many_arguments)]
    fn redeem_htlc(
        &self,
        secret: Secret,
        trade_id: TradeId,
        bob_success_address: L::Address,
        bob_success_keypair: KeyPair,
        alice_refund_address: L::Address,
        htlc_identifier: L::HtlcId,
        sell_amount: L::Quantity,
        lock_time: L::LockDuration,
    ) -> Result<L::TxId, Error>;

    fn create_query_to_watch_redeeming(&self, htlc_funding_tx_id: L::TxId) -> Result<Q, Error>;

    fn check_and_extract_secret(
        &self,
        create_htlc_tx_id: L::TxId,
        redeem_htlc_tx_id: L::TxId,
    ) -> Result<Secret, Error>;
}
