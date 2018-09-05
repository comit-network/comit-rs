use common_types::{
    ledger::Ledger,
    secret::{Secret, SecretHash},
};
use secp256k1_support::KeyPair;
use swaps::common::TradeId;

#[derive(Debug)]
pub enum Error {
    Unlocking,
    NodeConnection,
    Internal,
}

pub trait LedgerHtlcService<B: Ledger>: Send + Sync {
    fn deploy_htlc(
        &self,
        refund_address: B::Address,
        success_address: B::Address,
        time_lock: B::LockDuration,
        amount: B::Quantity,
        secret: SecretHash,
    ) -> Result<B::TxId, Error>;

    fn redeem_htlc(
        &self,
        secret: Secret,
        trade_id: TradeId,
        bob_success_address: B::Address,
        bob_success_keypair: KeyPair,
        client_refund_address: B::Address,
        htlc_identifier: B::HtlcId,
        sell_amount: B::Quantity,
        lock_time: B::LockDuration,
    ) -> Result<B::TxId, Error>;
}
