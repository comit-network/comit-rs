use common_types::ledger::Ledger;

#[derive(Debug, PartialEq)]
pub struct Request<SL: Ledger, TL: Ledger, SA, TA> {
    pub source_asset: SA,
    pub target_asset: TA,
    pub source_ledger: SL,
    pub target_ledger: TL,
    pub source_ledger_refund_pubkey: SL::Pubkey,
    pub target_ledger_success_pubkey: TL::Pubkey,
    pub source_ledger_lock_duration: SL::LockDuration,
    pub secret_hash: String,
}

impl<SL: Ledger, TL: Ledger, SA, TA> Request<SL, TL, SA, TA> {
    pub fn new(
        source_ledger: SL,
        target_ledger: TL,
        source_asset: SA,
        target_asset: TA,
        body: RequestBody<SL, TL>,
    ) -> Self {
        Request {
            source_ledger,
            source_asset,
            target_ledger,
            target_asset,
            source_ledger_refund_pubkey: body.source_ledger_refund_pubkey,
            target_ledger_success_pubkey: body.target_ledger_success_pubkey,
            source_ledger_lock_duration: body.source_ledger_lock_duration,
            secret_hash: body.secret_hash,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcceptResponse<SL: Ledger, TL: Ledger> {
    pub target_ledger_refund_pubkey: TL::Pubkey,
    pub source_ledger_success_pubkey: SL::Pubkey,
    pub target_ledger_lock_duration: TL::LockDuration,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct RequestBody<SL: Ledger, TL: Ledger> {
    pub source_ledger_refund_pubkey: SL::Pubkey,
    pub target_ledger_success_pubkey: TL::Pubkey,
    pub source_ledger_lock_duration: SL::LockDuration,
    pub secret_hash: String,
}
