use crate::swap_protocols::rfc003::{Ledger, SecretHash, Timestamp};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request<AL: Ledger, BL: Ledger, AA, BA> {
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AcceptResponseBody<AL: Ledger, BL: Ledger> {
    pub beta_ledger_refund_identity: BL::Identity,
    pub alpha_ledger_redeem_identity: AL::Identity,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct RequestBody<AL: Ledger, BL: Ledger> {
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}
