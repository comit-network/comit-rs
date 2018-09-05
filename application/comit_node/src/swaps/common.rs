use bitcoin_fee_service;
use bitcoin_rpc_client;
use event_store;
use ledger_htlc_service;
use logging;
use reqwest;
use rocket::{http::RawStr, request::FromParam};
use std::fmt;
use uuid::{self, Uuid};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TradeId(Uuid);

impl TradeId {
    pub fn new() -> TradeId {
        TradeId(Uuid::new_v4())
    }
}

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

#[derive(Debug)] //TODO merge these errors into error
pub enum Error {
    EventStore(event_store::Error),
    FeeService(bitcoin_fee_service::Error),
    LedgerHtlcService(ledger_htlc_service::Error),
    BitcoinRpc(bitcoin_rpc_client::RpcError),
    BitcoinNode(reqwest::Error),
    Unlocking(String),
}
