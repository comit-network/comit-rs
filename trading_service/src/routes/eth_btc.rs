use bitcoin_rpc;
use event_store;
use event_store::EventStore;
use event_store::OfferCreated;
use event_store::TradeAccepted;
use event_store::TradeCreated;
use exchange_api_client::ApiClient;
use exchange_api_client::ExchangeApiUrl;
use exchange_api_client::Offer;
use exchange_api_client::*;
use rand::OsRng;
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use secret::Secret;
use std::sync::Mutex;
use stub::BtcBlockHeight;
use stub::BtcHtlc;
use stub::EthAddress;
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
    client_success_address: EthAddress,
    client_refund_address: bitcoin_rpc::Address,
}

#[derive(Serialize, Deserialize)]
pub struct RequestToFund {
    uid: Uuid,
    address_to_fund: bitcoin_rpc::Address,
    //TODO: specify amount of BTC
}

const BTC_BLOCKS_IN_24H: BtcBlockHeight = BtcBlockHeight(24 * 60 / 10);

impl From<event_store::Error> for BadRequest<String> {
    fn from(_: event_store::Error) -> Self {
        BadRequest(None)
    }
}

#[post("/trades/ETH-BTC/<trade_id>/buy-orders", format = "application/json",
       data = "<buy_order_request_body>")]
pub fn post_buy_orders(
    trade_id: &RawStr,
    buy_order_request_body: Json<BuyOrderRequestBody>,
    url: State<ExchangeApiUrl>,
    event_store: State<EventStore>,
    rng: State<Mutex<OsRng>>,
) -> Result<Json<RequestToFund>, BadRequest<String>> {
    let trade_id = match Uuid::parse_str(trade_id.as_ref()) {
        Ok(trade_id) => trade_id,
        Err(_) => return Err(BadRequest(Some("Invalid trade id".to_string()))),
    };

    let offer = match event_store.get_offer_created(&trade_id) {
        Some(offer) => offer,
        None => return Err(BadRequest(None)),
    };

    let buy_order = buy_order_request_body.into_inner();
    let client_success_address = buy_order.client_success_address;
    let client_refund_address = buy_order.client_refund_address;

    let mut secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };

    let long_relative_timelock = BTC_BLOCKS_IN_24H;

    let trade_created_event = TradeCreated {
        uid: trade_id,
        secret: secret.clone(),
        client_success_address: client_success_address.clone(),
        client_refund_address: client_refund_address.clone(),
        long_relative_timelock: long_relative_timelock.clone(),
    };

    event_store.store_trade_created(trade_created_event.clone())?;

    let client = create_client(url.inner());

    let res = client.create_trade(
        offer.symbol,
        &TradeRequestBody {
            uid: trade_id,
            secret_hash: secret.hash().clone(),
            client_refund_address: client_refund_address.clone(),
            client_success_address: client_success_address.clone(),
            long_relative_timelock: long_relative_timelock.clone(),
        },
    );

    let trade_acceptance = match res {
        Ok(trade_acceptance) => trade_acceptance,
        Err(_) => return Err(BadRequest(None)), //TODO: handle error properly
    };

    let htlc = BtcHtlc::new(
        trade_acceptance.exchange_success_address.clone(),
        client_refund_address,
        long_relative_timelock,
    );

    let trade_accepted_event = TradeAccepted {
        uid: trade_id,
        short_relative_timelock: trade_acceptance.short_relative_timelock,
        exchange_refund_address: trade_acceptance.exchange_refund_address,
        exchange_success_address: trade_acceptance.exchange_success_address,
        htlc: htlc.clone(),
    };

    event_store.store_trade_accepted(trade_accepted_event)?;

    Ok(Json(RequestToFund {
        uid: trade_id,
        address_to_fund: htlc.address(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use exchange_api_client::ExchangeApiUrl;
    use rocket;
    use rocket::http::*;
    use rocket_factory::create_rocket_instance;
    use serde_json;

    #[test]
    fn happy_path_sell_x_btc_for_eth() {
        let url = ExchangeApiUrl("stub".to_string());

        let rocket = create_rocket_instance(url);
        let client = rocket::local::Client::new(rocket).unwrap();

        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(r#"{ "amount": 43 }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);
        let offer_response =
            serde_json::from_str::<Offer>(&response.body_string().unwrap()).unwrap();

        assert_eq!(
            offer_response.symbol,
            Symbol("ETH-BTC".to_string()),
            "offer_response has correct symbol"
        );
        let uid = offer_response.uid;

        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            // some random addresses I pulled off the internet
            .body(r#"{ "client_success_address": "0x4a965b089f8cb5c75efaa0fbce27ceaaf7722238", "client_refund_address" : "18wFjPJZRsYCn1vixJ1SwibS1wGCqB1YhT" }"#);

        let mut response = request.dispatch();

        assert_eq!(response.status(), Status::Ok);

        let trade =
            serde_json::from_str::<RequestToFund>(&response.body_string().unwrap()).unwrap();

        assert_eq!(
            trade.uid, uid,
            "UID for the funding request is the same as the offer response"
        );
    }

}
