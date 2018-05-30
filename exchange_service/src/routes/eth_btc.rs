use bitcoin_rpc;
use event_store::BtcBlockHeight;
use event_store::EthAddress;
use event_store::EthTimestamp;
use event_store::EventStore;
use event_store::OfferAccepted;
use event_store::OfferCreated;
pub use event_store::OfferCreated as OfferRequestResponse;
use event_store::OfferState;
use event_store::SecretHash;
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use rocket_factory::TreasuryApiUrl;
use treasury_api_client::{create_client, ApiClient, Symbol};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequestBody {
    pub amount: u32,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
fn post_buy_offers(
    offer_request_body: Json<OfferRequestBody>,
    event_store: State<EventStore>,
    treasury_api_url: State<TreasuryApiUrl>,
) -> Result<Json<OfferState>, BadRequest<String>> {
    // Request rate
    // Generate identifier
    // Store offer locally
    // Return offers (rate + expiry timestamp + exchange success address)

    let offer_request_body = offer_request_body.into_inner();

    let client = create_client(treasury_api_url.inner());
    let res = client.request_rate(Symbol("ETH-BTC".to_string()));
    let rate = match res {
        Ok(rate) => rate,
        Err(e) => {
            error!("{:?}", e);
            return Err(BadRequest(None));
        }
    };

    let uid = Uuid::new_v4();
    let offer_event = OfferCreated {
        uid,
        symbol: rate.symbol,
        amount: offer_request_body.amount,
        rate: rate.rate,
    };

    match event_store.store_offer(offer_event.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{}", e);
            return Err(BadRequest(None));
        }
    }

    Ok(Json(offer_event.clone())) // offer_event is the same than state.
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeRequestBody {
    pub secret_hash: SecretHash,
    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: EthAddress,
    pub long_relative_time_lock: BtcBlockHeight,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TradeRequestResponse {
    pub uid: Uuid,
    pub exchange_refund_address: EthAddress,
    pub exchange_success_address: bitcoin_rpc::Address,
    pub short_relative_time_lock: EthTimestamp,
}

impl From<OfferAccepted> for TradeRequestResponse {
    fn from(offer: OfferAccepted) -> Self {
        TradeRequestResponse {
            uid: offer.uid.clone(),
            exchange_refund_address: offer.exchange_refund_address.clone(),
            exchange_success_address: offer.exchange_success_address.clone(),
            short_relative_time_lock: offer.short_relative_time_lock.clone(),
        }
    }
}

#[post("/trades/ETH-BTC/<trade_id>/buy-orders", format = "application/json",
       data = "<trade_request_body>")]
pub fn post_buy_orders(
    trade_id: &RawStr,
    trade_request_body: Json<TradeRequestBody>,
    event_store: State<EventStore>,
    _treasury_api_url: State<TreasuryApiUrl>,
) -> Result<Json<TradeRequestResponse>, BadRequest<String>> {
    // Receive trade information
    // - Hashed Secret
    // - Client refund address (BTC)
    // - timeout (BTC)
    // - Client success address (ETH)
    // = generates exchange refund address
    // -> returns ETH HTLC data (exchange refund address + ETH timeout)

    let trade_request_body: TradeRequestBody = trade_request_body.into_inner();
    let uid = match Uuid::parse_str(trade_id.as_str()) {
        Ok(uid) => uid,
        Err(e) => {
            error!("{}", e);
            return Err(BadRequest(Some(format!("Error when parsing uid: {}", e))));
        }
    };

    // TODO: need to lock on uid now.

    // TODO: retrieve and use real address
    // This should never be used. Private key is: '9774cd25996588ef4bace0984eac1a80a3897c0cd3eea9858a6063c74f59e08b'
    let exchange_refund_address =
        EthAddress("0x1084d2C416fcc39564a4700a9B231270d463C5eA".to_string());

    let offer = OfferAccepted {
        uid,
        secret_hash: trade_request_body.secret_hash,
        client_refund_address: trade_request_body.client_refund_address,
        long_relative_time_lock: trade_request_body.long_relative_time_lock,
        short_relative_time_lock: EthTimestamp(12), //TODO: this is obviously not "12" :)
        client_success_address: trade_request_body.client_success_address,
        exchange_refund_address: exchange_refund_address.clone(),
        // TODO: retrieve and use real address
        // This should never be used. Private key is: 'cSVXkgbkkkjzXV2JMg1zWui4A4dCj55sp9hFoVSUQY9DVh9WWjuj'
        exchange_success_address: bitcoin_rpc::Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
    };

    match event_store.store_accepted_offer(offer.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            // TODO: create a to_string for e to return something nice.
            return Err(BadRequest(Some(e.to_string())));
        }
    }

    Ok(Json(offer.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::{Client, LocalResponse};
    use rocket_factory::create_rocket_instance;
    use serde_json;

    fn request_offer(client: &mut Client) -> LocalResponse {
        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(r#"{ "amount": 42 }"#);
        request.dispatch()
    }

    fn request_trade(client: &mut Client, uid: Uuid) -> LocalResponse {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "secret_hash": "MySecretHash",
                    "client_refund_address": "ClientRefundAddressInBtc",
                    "client_success_address": "0xClientSuccessAddressInEth",
                    "long_relative_time_lock": 24
                  }"#,
            );
        request.dispatch()
    }

    #[test]
    fn given_an_offer_request_then_return_valid_offer_response() {
        let url = TreasuryApiUrl("stub".to_string());
        let event_store = EventStore::new();

        let rocket = create_rocket_instance(url, event_store);
        let mut client = rocket::local::Client::new(rocket).unwrap();

        let mut response = request_offer(&mut client);
        assert_eq!(response.status(), Status::Ok);

        let offer_response =
            serde_json::from_str::<serde_json::Value>(&response.body_string().unwrap()).unwrap();
        assert_eq!(
            offer_response["symbol"], "ETH-BTC",
            "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
            offer_response
        );
    }

    #[test]
    fn given_a_trade_request_when_buy_offer_was_done_then_return_valid_trade_response() {
        let url = TreasuryApiUrl("stub".to_string());
        let event_store = EventStore::new();

        let rocket = create_rocket_instance(url, event_store);
        let mut client = rocket::local::Client::new(rocket).unwrap();

        let uid = {
            let mut response = request_offer(&mut client);
            assert_eq!(response.status(), Status::Ok);

            let offer_response =
                serde_json::from_str::<serde_json::Value>(&response.body_string().unwrap())
                    .unwrap();
            assert_eq!(
                offer_response["symbol"], "ETH-BTC",
                "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
                offer_response
            );

            Uuid::parse_str(offer_response["uid"].as_str().unwrap()).unwrap()
        };

        {
            let mut response = request_trade(&mut client, uid);
            assert_eq!(response.status(), Status::Ok);

            let trade_response =
                serde_json::from_str::<serde_json::Value>(&response.body_string().unwrap())
                    .unwrap();
            assert!(
                (trade_response["short_relative_time_lock"].as_i64().unwrap() > 0),
                "Expected to receive a time-lock in response of trade_offer. Json Response:\n{:?}",
                trade_response
            );
        }
    }

    #[test]
    fn given_a_trade_without_offer_should_fail() {
        let url = TreasuryApiUrl("stub".to_string());
        let event_store = EventStore::new();

        let rocket = create_rocket_instance(url, event_store);
        let mut client = rocket::local::Client::new(rocket).unwrap();

        let uid = Uuid::new_v4();

        {
            let response = request_trade(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }

    #[test]
    fn given_two_trades_request_with_same_uid_should_fail() {
        let url = TreasuryApiUrl("stub".to_string());
        let event_store = EventStore::new();

        let rocket = create_rocket_instance(url, event_store);
        let mut client = rocket::local::Client::new(rocket).unwrap();

        let uid = {
            let mut response = request_offer(&mut client);
            assert_eq!(response.status(), Status::Ok);

            let offer_response =
                serde_json::from_str::<OfferRequestResponse>(&response.body_string().unwrap())
                    .unwrap();
            assert_eq!(
                offer_response.symbol,
                Symbol("ETH-BTC".to_string()),
                "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
                offer_response
            );

            offer_response.uid.clone()
        };

        {
            let mut response = request_trade(&mut client, uid);
            assert_eq!(response.status(), Status::Ok);

            let trade_response =
                serde_json::from_str::<TradeRequestResponse>(&response.body_string().unwrap())
                    .unwrap();
            assert!(
                (trade_response.short_relative_time_lock > EthTimestamp(0)),
                "Expected to receive a time-lock in response of trade_offer. Json Response:\n{:?}",
                trade_response
            );
        }

        {
            let response = request_trade(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }
}
