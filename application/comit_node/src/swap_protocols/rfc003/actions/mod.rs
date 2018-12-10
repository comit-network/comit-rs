pub mod alice;
pub mod bob;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Action<Accept, Decline, Deploy, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund>
    Action<Accept, Decline, Deploy, Fund, Redeem, Refund>
{
    pub fn name(&self) -> String {
        use self::Action::*;
        match *self {
            Accept(_) => String::from("accept"),
            Decline(_) => String::from("decline"),
            Deploy(_) => String::from("deploy"),
            Fund(_) => String::from("fund"),
            Redeem(_) => String::from("redeem"),
            Refund(_) => String::from("refund"),
        }
    }
}

pub trait Actions {
    type ActionKind;

    fn actions(&self) -> Vec<Self::ActionKind>;
}
