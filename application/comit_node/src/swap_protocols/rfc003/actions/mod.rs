use crate::swap_protocols::{
    asset::Asset,
    rfc003::{secret_source::SecretSource, state_machine::HtlcParams, Ledger, Secret, Timestamp},
};

pub mod bitcoin;
pub mod erc20;
pub mod ether;

pub trait Actions {
    type ActionKind;

    fn actions(&self) -> Vec<Action<Self::ActionKind>>;
}

#[derive(Debug)]
pub struct Action<ActionKind> {
    pub invalid_until: Option<Timestamp>,
    pub inner: ActionKind,
}

impl<ActionKind> Action<ActionKind> {
    pub fn with_invalid_until(self, invalid_until: Timestamp) -> Self {
        Action {
            invalid_until: Some(invalid_until),
            ..self
        }
    }
}

pub trait FundAction<L: Ledger, A: Asset> {
    type FundActionOutput;

    fn fund_action(htlc_params: HtlcParams<L, A>) -> Self::FundActionOutput;
}

pub trait RefundAction<L: Ledger, A: Asset> {
    type RefundActionOutput;

    fn refund_action(
        htlc_params: HtlcParams<L, A>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn SecretSource,
    ) -> Self::RefundActionOutput;
}

pub trait RedeemAction<L: Ledger, A: Asset> {
    type RedeemActionOutput;

    fn redeem_action(
        htlc_params: HtlcParams<L, A>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn SecretSource,
        secret: Secret,
    ) -> Self::RedeemActionOutput;
}
