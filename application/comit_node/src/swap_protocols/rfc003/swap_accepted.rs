use crate::swap_protocols::{
    asset::Asset,
    rfc003::{self, ledger::Ledger, state_machine::HtlcParams},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwapAccepted<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> {
    pub request: rfc003::messages::Request<AL, BL, AA, BA>,
    pub alpha_redeem_identity: AL::Identity,
    pub beta_refund_identity: BL::Identity,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> SwapAccepted<AL, BL, AA, BA> {
    pub fn alpha_htlc_params(&self) -> HtlcParams<AL, AA> {
        HtlcParams {
            asset: self.request.alpha_asset.clone(),
            ledger: self.request.alpha_ledger.clone(),
            redeem_identity: self.alpha_redeem_identity,
            refund_identity: self.request.alpha_ledger_refund_identity,
            expiry: self.request.alpha_expiry,
            secret_hash: self.request.secret_hash,
        }
    }
    pub fn beta_htlc_params(&self) -> HtlcParams<BL, BA> {
        HtlcParams {
            asset: self.request.beta_asset.clone(),
            ledger: self.request.beta_ledger.clone(),
            redeem_identity: self.request.beta_ledger_redeem_identity,
            refund_identity: self.beta_refund_identity,
            expiry: self.request.beta_expiry,
            secret_hash: self.request.secret_hash,
        }
    }
}
