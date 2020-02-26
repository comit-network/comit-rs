pub mod bitcoin;
pub mod erc20;
pub mod ether;

use crate::{
    asset::Asset,
    swap_protocols::rfc003::{create_swap::HtlcParams, DeriveIdentities, Ledger, Secret},
};
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

pub trait FundAction<L, A>
where
    L: Ledger,
    A: Asset,
{
    type FundActionOutput;

    fn fund_action(htlc_params: HtlcParams<'_, L, A, L::Identity>) -> Self::FundActionOutput;
}

pub trait RefundAction<L, A>
where
    L: Ledger,
    A: Asset,
{
    type RefundActionOutput;

    fn refund_action(
        htlc_params: HtlcParams<'_, L, A, L::Identity>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn DeriveIdentities,
        fund_transaction: &L::Transaction,
    ) -> Self::RefundActionOutput;
}

pub trait RedeemAction<L, A>
where
    L: Ledger,
    A: Asset,
{
    type RedeemActionOutput;

    fn redeem_action(
        htlc_params: HtlcParams<'_, L, A, L::Identity>,
        htlc_location: L::HtlcLocation,
        secret_source: &dyn DeriveIdentities,
        secret: Secret,
    ) -> Self::RedeemActionOutput;
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
