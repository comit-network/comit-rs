mod btc_erc20;
mod btc_eth;
mod eth_btc;

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ActionKind<Deploy, Fund, Redeem, Refund> {
    Deploy(Deploy),
    Fund(Fund),
    Redeem(Redeem),
    Refund(Refund),
}

impl<Deploy, Fund, Redeem, Refund> ActionKind<Deploy, Fund, Redeem, Refund> {
    pub fn name(&self) -> String {
        use self::ActionKind::*;
        match *self {
            Deploy(_) => String::from("deploy"),
            Fund(_) => String::from("fund"),
            Redeem(_) => String::from("redeem"),
            Refund(_) => String::from("refund"),
        }
    }
}
