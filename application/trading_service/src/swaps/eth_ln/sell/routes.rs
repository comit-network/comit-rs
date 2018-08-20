use common_types::TradingSymbol;
use event_store::{self, EventStore, InMemoryEventStore};
use exchange_api_client::{ApiClient, OfferResponseBody};
use reqwest;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swaps::{events::OfferCreated, TradeId};

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
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let symbol = TradingSymbol::ETH_LN;
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
    offer_request_body: OfferRequestBody,
    symbol: TradingSymbol,
) -> Result<OfferResponseBody, Error> {
    let offer = client
        .create_buy_offer(symbol, offer_request_body.amount)
        .map_err(Error::ExchangeService)?;
    let id = offer.uid.clone();
    //TODO Fixme, this feature has a lower priority for now. For this we need decide either on
    // how we handle Lightning, i.e. creating an additional ledger for Lightning payments
    //    let event = OfferCreated::from(offer.clone());
    //
    //    event_store.add_event(id, event)?;
    Ok(offer)
}
