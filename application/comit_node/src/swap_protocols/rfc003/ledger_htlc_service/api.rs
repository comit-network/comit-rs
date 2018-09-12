use common_types::secret::{Secret, SecretHash};
use ganp::ledger::Ledger;
use secp256k1_support::KeyPair;
use swaps::common::TradeId;

#[derive(Debug)]
pub enum Error {
    Unlocking,
    NodeConnection,
    Internal,
}

pub trait LedgerHtlcService<B: Ledger, H>: Send + Sync {
    fn deploy_htlc(&self, htlc_params: H) -> Result<B::TxId, Error>;

    fn redeem_htlc(
        &self,
        secret: Secret,
        trade_id: TradeId,
        bob_success_address: B::Address,
        bob_success_keypair: KeyPair,
        alice_refund_address: B::Address,
        htlc_identifier: B::HtlcId,
        sell_amount: B::Quantity,
        lock_time: B::LockDuration,
    ) -> Result<B::TxId, Error>;
}
