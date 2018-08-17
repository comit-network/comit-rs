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

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: f64,
}

#[post("/trades/ETH-LN/sell-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_sell_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();
    let symbol = TradingSymbol::ETH_LN;

    let res = client.create_offer(symbol, offer_request_body.amount);

    match res {
        Ok(offer) => {
            let id = offer.uid.clone();
            let event = OfferCreated::from(offer.clone());

            event_store.add_event(id, event).map_err(Error::EventStore)?;
            Ok(Json(offer))
        }
        Err(e) => {
            error!("{:?}", e);

            Err(BadRequest(None))
        }
    }
}
