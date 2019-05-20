use crate::{
    http_api::route_factory::swap_path,
    swap_protocols::{
        rfc003::{alice, bob},
        SwapId,
    },
};
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

pub trait ToActionName {
    fn to_action_name(&self) -> ActionName;
}

impl<Deploy, Fund, Redeem, Refund> ToActionName
    for alice::ActionKind<Deploy, Fund, Redeem, Refund>
{
    fn to_action_name(&self) -> ActionName {
        use self::alice::ActionKind::*;
        match self {
            Deploy(_) => ActionName::Deploy,
            Fund(_) => ActionName::Fund,
            Redeem(_) => ActionName::Redeem,
            Refund(_) => ActionName::Refund,
        }
    }
}

impl<Accept, Decline, Deploy, Fund, Redeem, Refund> ToActionName
    for bob::ActionKind<Accept, Decline, Deploy, Fund, Redeem, Refund>
{
    fn to_action_name(&self) -> ActionName {
        use self::bob::ActionKind::*;
        match self {
            Accept(_) => ActionName::Accept,
            Decline(_) => ActionName::Decline,
            Deploy(_) => ActionName::Deploy,
            Fund(_) => ActionName::Fund,
            Redeem(_) => ActionName::Redeem,
            Refund(_) => ActionName::Refund,
        }
    }
}

pub trait ToSirenAction {
    fn to_siren_action(&self, name: String, href: String) -> siren::Action;
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

pub fn new_action_link(id: &SwapId, action: ActionName) -> String {
    format!("{}/{}", swap_path(*id), action)
}

#[cfg(test)]
mod test {
    use crate::http_api::routes::rfc003::GetActionQueryParams;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<GetActionQueryParams>(s);
        assert_eq!(res, Ok(GetActionQueryParams::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_byte=10.59";

        let res = serde_urlencoded::from_str::<GetActionQueryParams>(s);
        assert_eq!(
            res,
            Ok(GetActionQueryParams::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_byte: "10.59".to_string(),
            })
        );
    }
}
