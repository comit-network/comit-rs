pub mod bitcoin;
pub mod erc20;
pub mod ether;

use crate::swap_protocols::rfc003::{DeriveIdentities, Secret};
use std::marker::PhantomData;

/// Defines the set of actions available in the RFC003 protocol
#[derive(Debug, Clone, PartialEq, strum_macros::EnumDiscriminants)]
#[strum_discriminants(
    name(ActionKind),
    derive(Display, EnumString),
    strum(serialize_all = "snake_case")
)]
pub enum Action<Accept, Decline, Deploy, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

pub trait MakeFundAction {
    type HtlcParams;
    type Output;

    fn make_fund_action(htlc_params: Self::HtlcParams) -> Self::Output;
}

pub trait MakeRefundAction {
    type HtlcParams;
    type HtlcLocation;
    type FundTransaction;
    type Output;

    fn make_refund_action(
        htlc_params: Self::HtlcParams,
        htlc_location: Self::HtlcLocation,
        secret_source: &dyn DeriveIdentities,
        fund_transaction: &Self::FundTransaction,
    ) -> Self::Output;
}

pub trait MakeRedeemAction {
    type HtlcParams;
    type HtlcLocation;
    type Output;

    fn make_redeem_action(
        htlc_params: Self::HtlcParams,
        htlc_location: Self::HtlcLocation,
        secret_source: &dyn DeriveIdentities,
        secret: Secret,
    ) -> Self::Output;
}

#[derive(Clone, Debug, Default)]
pub struct Accept<AL, BL> {
    phantom_data: PhantomData<(AL, BL)>,
}

impl<AL, BL> Accept<AL, BL> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Decline<AL, BL> {
    phantom_data: PhantomData<(AL, BL)>,
}

impl<AL, BL> Decline<AL, BL> {
    pub fn new() -> Self {
        Self {
            phantom_data: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn action_kind_serializes_into_lowercase_str() {
        assert_eq!(ActionKind::Accept.to_string(), "accept".to_string());
        assert_eq!(ActionKind::Decline.to_string(), "decline".to_string());
        assert_eq!(ActionKind::Fund.to_string(), "fund".to_string());
        assert_eq!(ActionKind::Refund.to_string(), "refund".to_string());
        assert_eq!(ActionKind::Redeem.to_string(), "redeem".to_string());
        assert_eq!(ActionKind::Deploy.to_string(), "deploy".to_string());
    }
}
