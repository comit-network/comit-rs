use common_types::{ledger::Ledger, secret::SecretHash};

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
        time_lock: B::Time,
        amount: B::Quantity,
        secret: SecretHash,
    ) -> Result<B::TxId, Error>;
}
