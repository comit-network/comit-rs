use crate::swap_protocols::{
    asset::Asset,
    metadata_store::{Metadata, RoleKind},
    rfc003::{Ledger, SecretHash},
};

#[derive(Clone, Debug, PartialEq, LabelledGeneric)]
pub struct SwapRequest<AL: Ledger, BL: Ledger, AA, BA> {
    pub alpha_asset: AA,
    pub beta_asset: BA,
    pub alpha_ledger: AL,
    pub beta_ledger: BL,
    pub alpha_ledger_refund_identity: AL::Identity,
    pub beta_ledger_redeem_identity: BL::Identity,
    pub alpha_ledger_lock_duration: AL::LockDuration,
    pub secret_hash: SecretHash,
}

impl<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset> From<SwapRequest<AL, BL, AA, BA>> for Metadata {
    fn from(request: SwapRequest<AL, BL, AA, BA>) -> Self {
        Self {
            alpha_ledger: request.alpha_ledger.into(),
            beta_ledger: request.beta_ledger.into(),
            alpha_asset: request.alpha_asset.into(),
            beta_asset: request.beta_asset.into(),
            role: RoleKind::Bob,
        }
    }
}
