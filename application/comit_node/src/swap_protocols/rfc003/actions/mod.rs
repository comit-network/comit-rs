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

impl<Accept, Decline, Fund, Redeem, Refund> Action<Accept, Decline, Fund, Redeem, Refund> {
    pub fn name(&self) -> String {
        use self::Action::*;
        match *self {
            Accept(_) => String::from("accept"),
            Decline(_) => String::from("decline"),
            Fund(_) => String::from("fund"),
            Redeem(_) => String::from("redeem"),
            Refund(_) => String::from("refund"),
        }
    }
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
