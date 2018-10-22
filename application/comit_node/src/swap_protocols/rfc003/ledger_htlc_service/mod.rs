pub use self::{bitcoin_service::*, ethereum_service::*};
use swap_protocols::{ledger::Ledger, rfc003::Secret};
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

pub trait LedgerHtlcService<L: Ledger, H, R, Q>: Send + Sync {
    fn fund_htlc(&self, htlc_funding_params: H) -> Result<L::TxId, Error>;

    #[allow(clippy::too_many_arguments)]
    fn redeem_htlc(&self, trade_id: TradeId, htlc_redeem_params: R) -> Result<L::TxId, Error>;

    fn create_query_to_watch_redeeming(&self, htlc_funding_tx_id: L::TxId) -> Result<Q, Error>;

    fn create_query_to_watch_funding(&self, htlc_params: H) -> Q;

    fn check_and_extract_secret(
        &self,
        create_htlc_tx_id: L::TxId,
        redeem_htlc_tx_id: L::TxId,
    ) -> Result<Secret, Error>;
}
