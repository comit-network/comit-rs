use crate::swap_protocols::{
    asset::Asset,
    rfc003::{Ledger, SecretHash, SecretSource},
    HashFunction, Timestamp,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Request<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub hash_function: HashFunction,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_expiry: Timestamp,
    pub beta_expiry: Timestamp,
    pub secret_hash: SecretHash,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

pub trait ToRequest<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    fn to_request(&self, secret_source: &dyn SecretSource) -> Request<AL, BL, AA, BA>;
}

pub trait IntoAcceptResponseBody<AL: Ledger, BL: Ledger> {
    fn into_accept_response_body(
        self,
        secret_source: &dyn SecretSource,
    ) -> AcceptResponseBody<AL, BL>;
}
