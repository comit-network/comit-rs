use swap_protocols::rfc003::{Ledger, SecretHash};

#[derive(Clone, Debug, PartialEq)]
pub struct Request<SL: Ledger, TL: Ledger, SA, TA> {
    pub source_asset: SA,
    pub target_asset: TA,
    pub source_ledger: SL,
    pub target_ledger: TL,
    pub source_ledger_refund_identity: SL::Identity,
    pub target_ledger_success_identity: TL::Identity,
    pub source_ledger_lock_duration: SL::LockDuration,
    pub secret_hash: SecretHash,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AcceptResponseBody<SL: Ledger, TL: Ledger> {
    pub target_ledger_refund_identity: TL::Identity,
    pub source_ledger_success_identity: SL::Identity,
    pub target_ledger_lock_duration: TL::LockDuration,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RequestBody<SL: Ledger, TL: Ledger> {
    pub source_ledger_refund_identity: SL::Identity,
    pub target_ledger_success_identity: TL::Identity,
    pub source_ledger_lock_duration: SL::LockDuration,
    pub secret_hash: SecretHash,
}
