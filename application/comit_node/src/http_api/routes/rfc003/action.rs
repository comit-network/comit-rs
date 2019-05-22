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

pub trait ToSirenAction {
    fn to_siren_action(&self, id: &SwapId) -> siren::Action;
}

pub trait ListRequiredFields {
    fn list_required_fields() -> Vec<siren::Field>;
}

pub fn new_action_link(id: &SwapId, action: &str) -> String {
    format!("{}/{}", swap_path(*id), action)
}

#[derive(strum_macros::EnumString, strum_macros::Display, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Accept,
    Decline,
    Deploy,
    Fund,
    Refund,
    Redeem,
}

#[cfg(test)]
mod test {
    use crate::http_api::routes::rfc003::ActionExecutionParameters;

    #[test]
    fn given_no_query_parameters_deserialize_to_none() {
        let s = "";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(res, Ok(ActionExecutionParameters::None {}));
    }

    #[test]
    fn given_bitcoin_identity_and_fee_deserialize_to_ditto() {
        let s = "address=1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa&fee_per_byte=10.59";

        let res = serde_urlencoded::from_str::<ActionExecutionParameters>(s);
        assert_eq!(
            res,
            Ok(ActionExecutionParameters::BitcoinAddressAndFee {
                address: "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa".parse().unwrap(),
                fee_per_byte: "10.59".to_string(),
            })
        );
    }
}
