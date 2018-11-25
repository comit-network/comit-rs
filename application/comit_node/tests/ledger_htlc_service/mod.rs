pub use self::bitcoin_service::*;
use comit_node::{
    swap_protocols::{
        asset::Asset,
        rfc003::{Ledger, Secret},
    },
    swaps::common::SwapId,
};

mod bitcoin_service;

#[derive(Debug)]
pub enum Error {
    Unlocking,
    NodeConnection,
    Internal,
    TransactionNotFound,
    InvalidRedeemTransaction,
}

pub trait LedgerHtlcService<L: Ledger, A: Asset, H, R>: Send + Sync {
    fn deploy_htlc(&self, htlc_funding_params: H) -> Result<L::TxId, Error>;

    fn fund_htlc(&self, target: L::Address, asset: A) -> Result<L::TxId, Error>;

    #[allow(clippy::too_many_arguments)]
    fn redeem_htlc(&self, trade_id: SwapId, htlc_redeem_params: R) -> Result<L::TxId, Error>;

    fn check_and_extract_secret(
        &self,
        create_htlc_tx_id: L::TxId,
        redeem_htlc_tx_id: L::TxId,
    ) -> Result<Secret, Error>;
}
