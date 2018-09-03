use bitcoin_support;
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    TradingSymbol,
};
use ethereum_support;
use event_store::{self, InMemoryEventStore};
use reqwest;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use swaps::{OfferResponseBody, TradeId};
use uuid::Uuid;

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ExchangeService(reqwest::Error),
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

#[derive(Deserialize)]
pub struct OfferRequestBody {
    amount: f64,
}

#[post("/trades/ETH-LN/sell-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_sell_offers(
    offer_request_body: Json<OfferRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody<Bitcoin, Ethereum>>, BadRequest<String>> {
    let symbol = TradingSymbol::ETH_LN;
    let offer_response_body =
        handle_sell_offer(event_store.inner(), offer_request_body.into_inner(), symbol)?;

    Ok(Json(offer_response_body))
}

fn handle_sell_offer(
    _event_store: &InMemoryEventStore<TradeId>,
    offer_request_body: OfferRequestBody,
    symbol: TradingSymbol,
) -> Result<OfferResponseBody<Bitcoin, Ethereum>, Error> {
    let rate = 0.1; //TODO export this somewhere
    let sell_amount = offer_request_body.amount;
    let buy_amount = sell_amount * rate;

    let offer: OfferResponseBody<Bitcoin, Ethereum> = OfferResponseBody {
        uid: TradeId::from(Uuid::new_v4()),
        symbol,
        rate,
        sell_amount: ethereum_support::EthereumQuantity::from_eth(sell_amount),
        buy_amount: bitcoin_support::BitcoinQuantity::from_bitcoin(buy_amount),
    };

    let _id = offer.uid.clone();
    //TODO Fixme, this feature has a lower priority for now. For this we need decide either on
    // how we handle Lightning, i.e. creating an additional ledger for Lightning payments
    //    let event = OfferCreated::from(offer.clone());
    //
    //    event_store.add_event(id, event)?;
    Ok(offer)
}
