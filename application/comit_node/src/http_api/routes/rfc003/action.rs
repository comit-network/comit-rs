use crate::{
    http_api::route_factory::swap_path,
    swap_protocols::{
        rfc003::{alice, bob, Action},
        SwapId,
    },
};
use rustic_hal::HalLink;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionName {
    Accept,
    Decline,
    Fund,
    Deploy,
    Redeem,
    Refund,
}

// This should probably live one level up
pub trait ToActionName {
    fn to_action_name(&self) -> ActionName;
}

impl<Deploy, Fund, Redeem, Refund> ToActionName
    for Action<alice::ActionKind<Deploy, Fund, Redeem, Refund>>
{
    fn to_action_name(&self) -> ActionName {
        use self::alice::ActionKind::*;
        match self.inner {
            Deploy(_) => ActionName::Deploy,
            Fund(_) => ActionName::Fund,
            Redeem(_) => ActionName::Redeem,
            Refund(_) => ActionName::Refund,
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> ToActionName
    for Action<bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>>
{
    fn to_action_name(&self) -> ActionName {
        use self::bob::ActionKind::*;
        match self.inner {
            Accept(_) => ActionName::Accept,
            Decline(_) => ActionName::Decline,
            Deploy(_) => ActionName::Deploy,
            Fund(_) => ActionName::Fund,
            Redeem(_) => ActionName::Redeem,
            Refund(_) => ActionName::Refund,
        }
    }
}

#[derive(Debug)]
pub struct UnknownAction(String);

impl FromStr for ActionName {
    type Err = UnknownAction;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "accept" => ActionName::Accept,
            "decline" => ActionName::Decline,
            "fund" => ActionName::Fund,
            "deploy" => ActionName::Deploy,
            "redeem" => ActionName::Redeem,
            "refund" => ActionName::Refund,
            s => return Err(UnknownAction(s.to_string())),
        })
    }
}

impl From<ActionName> for String {
    fn from(action: ActionName) -> Self {
        action.to_string()
    }
}

impl Display for ActionName {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = match self {
            ActionName::Accept => "accept",
            ActionName::Decline => "decline",
            ActionName::Fund => "fund",
            ActionName::Deploy => "deploy",
            ActionName::Redeem => "redeem",
            ActionName::Refund => "refund",
        };
        write!(f, "{}", s)
    }
}

pub fn new_action_link(id: &SwapId, action: ActionName) -> HalLink {
    let route = format!("{}/{}", swap_path(*id), action);
    route.into()
}
