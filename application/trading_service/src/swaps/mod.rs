use logging;
use rocket::{http::RawStr, request::FromParam};
use std::fmt;
use uuid::{self, Uuid};

pub mod errors;
pub mod eth_btc;
pub mod eth_ln;
pub mod events;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl From<Uuid> for TradeId {
    fn from(uuid: Uuid) -> Self {
        TradeId(uuid)
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl<'a> FromParam<'a> for TradeId {
    type Error = uuid::ParseError;

    fn from_param(param: &RawStr) -> Result<Self, <Self as FromParam>::Error> {
        Uuid::parse_str(param.as_str()).map(|uid| {
            logging::set_context(&uid);
            TradeId::from(uid)
        })
    }
}
