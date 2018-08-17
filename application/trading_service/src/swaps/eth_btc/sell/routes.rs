use bitcoin_htlc::{self, Htlc as BtcHtlc};
use bitcoin_rpc::BlockHeight;
use bitcoin_support::{self, BitcoinQuantity, Network, PubkeyHash};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    TradingSymbol,
};
use ethereum_support::{self, EthereumQuantity};
use event_store::{self, EventStore, InMemoryEventStore};
use exchange_api_client::{ApiClient, OfferResponseBody, OrderRequestBody};
use rand::OsRng;
use reqwest;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use rustc_hex;
use secret::Secret;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};
use swaps::{
    events::{ContractDeployed, OfferCreated, OrderCreated, OrderTaken},
    TradeId,
};

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ExchangeService(reqwest::Error),
    TradingService(String),
}

impl From<Error> for BadRequest<String> {
    fn from(e: Error) -> Self {
        error!("{:?}", e);
        BadRequest(None)
    }
}

impl From<event_store::Error> for Error {
    fn from(e: event_store::Error) -> Self {
        Error::EventStore(e)
    }
}

impl From<bitcoin_support::Error> for Error {
    fn from(_e: bitcoin_support::Error) -> Self {
        Error::TradingService(String::from("Invalid address format"))
    }
}

impl From<rustc_hex::FromHexError> for Error {
    fn from(_e: rustc_hex::FromHexError) -> Self {
        Error::TradingService(String::from("Invalid address format"))
    }
}

#[derive(Deserialize)]
pub struct SellOfferRequestBody {
    amount: f64,
}

#[post("/trades/ETH-BTC/sell-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_sell_offers(
    offer_request_body: Json<SellOfferRequestBody>,
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let symbol = TradingSymbol::ETH_BTC;

    let offer_response_body = handle_sell_offer(
        client.inner(),
        event_store.inner(),
        offer_request_body.into_inner(),
        symbol,
    )?;

    Ok(Json(offer_response_body))
}

fn handle_sell_offer(
    client: &Arc<ApiClient>,
    event_store: &InMemoryEventStore<TradeId>,
    offer_request_body: SellOfferRequestBody,
    symbol: TradingSymbol,
) -> Result<OfferResponseBody, Error> {
    let offer = client
        .create_sell_offer(symbol, offer_request_body.amount)
        .map_err(Error::ExchangeService)?;
    let id = offer.uid.clone();
    let event = OfferCreated::from(offer.clone());

    event_store.add_event(id, event)?;
    Ok(offer)
}
