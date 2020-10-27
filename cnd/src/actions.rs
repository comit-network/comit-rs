pub use comit::actions::*;

/// Describes how to get the `fund` action from the current state.
///
/// If `fund` is not feasible in the current state, this should return `None`.
pub trait FundAction {
    type Output;

    fn fund_action(&self) -> anyhow::Result<Self::Output>;
}

pub trait DeployAction {
    type Output;

    fn deploy_action(&self) -> anyhow::Result<Self::Output>;
}

/// Describes how to get the `redeem` action from the current state.
///
/// If `redeem` is not feasible in the current state, this should return `None`.
pub trait RedeemAction {
    type Output;

    fn redeem_action(&self, btc_per_vbyte: bitcoin::Amount) -> anyhow::Result<Self::Output>;
}

/// Describes how to get the `refund` action from the current state.
///
/// If `refund` is not feasible in the current state, this should return `None`.
pub trait RefundAction {
    type Output;

    fn refund_action(&self, btc_per_vbyte: bitcoin::Amount) -> anyhow::Result<Self::Output>;
}
