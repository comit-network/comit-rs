pub mod alice;
pub mod bob;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Action<Accept, Decline, AddInvoice, Deploy, Fund, Redeem, Refund> {
    Accept(Accept),
    Decline(Decline),
    AddInvoice(AddInvoice),
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

impl<Accept, Decline, AddInvoice, Deploy, Fund, Redeem, Refund>
    Action<Accept, Decline, AddInvoice, Deploy, Fund, Redeem, Refund>
{
    pub fn name(&self) -> String {
        use self::Action::*;
        match *self {
            Accept(_) => String::from("accept"),
            Decline(_) => String::from("decline"),
            AddInvoice(_) => String::from("addInvoice"),
            Deploy(_) => String::from("deploy"),
            Fund(_) => String::from("fund"),
            Redeem(_) => String::from("redeem"),
            Refund(_) => String::from("refund"),
        }
    }
}

pub trait StateActions {
    type Accept;
    type Decline;
    type AddInvoice;
    type Deploy;
    type Fund;
    type Redeem;
    type Refund;

    fn actions(
        &self,
    ) -> Vec<
        Action<
            Self::Accept,
            Self::Decline,
            Self::AddInvoice,
            Self::Deploy,
            Self::Fund,
            Self::Redeem,
            Self::Refund,
        >,
    >;
}
