use common_types::secret::SecretHash;
use ledger::Ledger;
use swap::{self, SwapProtocol, SwapRequestHeaders};

#[derive(Debug, PartialEq)]
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

impl<SL: Ledger, TL: Ledger, SA: Into<swap::Asset>, TA: Into<swap::Asset>> Request<SL, TL, SA, TA> {
    pub fn into_headers_and_body(self) -> (SwapRequestHeaders, RequestBody<SL, TL>) {
        (
            SwapRequestHeaders {
                source_ledger: self.source_ledger.into(),
                target_ledger: self.target_ledger.into(),
                source_asset: self.source_asset.into(),
                target_asset: self.target_asset.into(),
                swap_protocol: SwapProtocol::ComitRfc003,
            },
            RequestBody {
                source_ledger_refund_identity: self.source_ledger_refund_identity,
                target_ledger_success_identity: self.target_ledger_success_identity,
                source_ledger_lock_duration: self.source_ledger_lock_duration,
                secret_hash: self.secret_hash,
            },
        )
    }
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
            source_ledger_refund_identity: body.source_ledger_refund_identity,
            target_ledger_success_identity: body.target_ledger_success_identity,
            source_ledger_lock_duration: body.source_ledger_lock_duration,
            secret_hash: body.secret_hash,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AcceptResponse<SL: Ledger, TL: Ledger> {
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
