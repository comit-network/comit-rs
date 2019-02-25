use crate::{
    http_api::route_factory::swap_path,
    swap_protocols::{
        rfc003::{alice, bob},
        SwapId,
    },
};
use rustic_hal::HalResource;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Accept,
    Decline,
    Fund,
    Deploy,
    Redeem,
    Refund,
}

// This should probably live one level up
pub trait ToAction {
    fn to_action(&self) -> Action;
}

impl<Deploy, Fund, Redeem, Refund> ToAction for alice::ActionKind<Deploy, Fund, Redeem, Refund> {
    fn to_action(&self) -> Action {
        use self::alice::ActionKind::*;
        match *self {
            Deploy(_) => Action::Deploy,
            Fund(_) => Action::Fund,
            Redeem(_) => Action::Redeem,
            Refund(_) => Action::Refund,
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> ToAction
    for bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
{
    fn to_action(&self) -> Action {
        use self::bob::ActionKind::*;
        match *self {
            Accept(_) => Action::Accept,
            Decline(_) => Action::Decline,
            Deploy(_) => Action::Deploy,
            Fund(_) => Action::Fund,
            Redeem(_) => Action::Redeem,
            Refund(_) => Action::Refund,
        }
    }
}

#[derive(Debug)]
pub struct UnknownAction(String);

impl FromStr for Action {
    type Err = UnknownAction;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "accept" => Action::Accept,
            "decline" => Action::Decline,
            "fund" => Action::Fund,
            "deploy" => Action::Deploy,
            "redeem" => Action::Redeem,
            "refund" => Action::Refund,
            s => return Err(UnknownAction(s.to_string())),
        })
    }
}

impl Display for Action {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = match self {
            Action::Accept => "accept",
            Action::Decline => "decline",
            Action::Fund => "fund",
            Action::Deploy => "deploy",
            Action::Redeem => "redeem",
            Action::Refund => "refund",
        };
        write!(f, "{}", s)
    }
}

// SwapId may not be the right type to use here
pub trait AddLinks<T> {
    fn add_links(&mut self, id: &SwapId, links: Vec<T>);
}

impl AddLinks<Action> for HalResource {
    fn add_links(&mut self, id: &SwapId, actions: Vec<Action>) {
        for action in actions {
            let route = format!("{}/{}", swap_path(*id), action);
            self.with_link(action.to_string(), route);
        }
    }
}
