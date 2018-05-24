use event_store::EventStore;
use event_store::OfferCreated;
use exchange_api_client::ApiClient;
use exchange_api_client::ExchangeApiUrl;
use exchange_api_client::Offer;
use exchange_api_client::*;
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use symbol::Symbol;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: u32,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    url: State<ExchangeApiUrl>,
    event_store: State<EventStore>,
) -> Result<Json<Offer>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();
    let symbol = Symbol("ETH-BTC".to_string());

    let client = create_client(url.inner());

    let res = client.create_offer(symbol, offer_request_body.amount);

    match res {
        Ok(offer) => {
            event_store.store_offer_created(OfferCreated::from(offer.clone()));

            Ok(Json(offer))
        }
        Err(e) => {
            error!("{:?}", e);

            Err(BadRequest(None))
        }
    }
}

#[derive(Deserialize)]
pub struct BuyOrderRequestBody {
    client_success_address: String,
    client_refund_address: String,
}

#[post("/trades/ETH-BTC/<trade_id>/buy-orders", format = "application/json",
       data = "<buy_order_request_body>")]
pub fn post_buy_orders(
    trade_id: &RawStr,
    buy_order_request_body: Json<BuyOrderRequestBody>,
    url: State<ExchangeApiUrl>,
    event_store: State<EventStore>,
) -> Result<Json<()>, BadRequest<String>> {
    // pull offer for trade from DB
    // generate secret
    // generate HTLC
    // secret and HTLC in DB
    // send stuff to exchange

    if let Ok(trade_id) = Uuid::parse_str(trade_id.as_ref()) {
        if let Some(offer) = event_store.get_offer_created(&trade_id) {
            Err(BadRequest(None))
        } else {
            Err(BadRequest(None))
        }
    } else {
        Err(BadRequest(Some("Invalid trade id".to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_rpc::Address;
    use exchange_api_client::ExchangeApiUrl;
    use rocket;
    use rocket::http::*;
    use rocket_factory::create_rocket_instance;
    use serde_json;

    #[test]
    fn given_an_offer_from_exchange_should_respond_with_offer() {
        let url = ExchangeApiUrl("stub".to_string());
        let event_store = EventStore::new();

        let rocket = create_rocket_instance(url, event_store);
        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(r#"{ "amount": 43 }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let offer_response =
            serde_json::from_str::<Offer>(&response.body_string().unwrap()).unwrap();

        assert_eq!(offer_response.symbol, Symbol("ETH-BTC".to_string()));
    }

}
