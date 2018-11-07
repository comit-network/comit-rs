pub mod alice;
pub mod bob;

mod bitcoin;
mod ethereum;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Action<Accept, Decline, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

pub trait StateActions {
    type Accept;
    type Decline;
    type Fund;
    type Redeem;
    type Refund;

    fn actions(
        &self,
    ) -> Vec<Action<Self::Accept, Self::Decline, Self::Fund, Self::Redeem, Self::Refund>>;
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Accept;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Decline;
