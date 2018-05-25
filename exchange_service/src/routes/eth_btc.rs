use bitcoin_rpc;
use event_store::{EventStore, OfferState, TradeEvent};
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use treasury_api_client::{create_client, ApiClient};
use types::{BtcBlockHeight, EthAddress, EthTimestamp};
use types::{SecretHash, Symbol, TreasuryApiUrl};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequestBody {
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferRequestResponse {
    pub uid: Uuid,
    pub symbol: Symbol,
    pub amount: u32,
    pub rate: f32,
    pub exchange_success_address: bitcoin_rpc::Address,
    // TODO: treasury_expiry_timestamp
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
            return Err(BadRequest(Some(e.to_string())));
        }
    };

    let uid = Uuid::new_v4();
    let offer_event = OfferRequestResponse {
        uid,
        symbol: rate.symbol,
        amount: offer_request_body.amount,
        rate: rate.rate,
        // TODO: retrieve and use real address
        // This should never be used. Private key is: 'cSVXkgbkkkjzXV2JMg1zWui4A4dCj55sp9hFoVSUQY9DVh9WWjuj'
        // TODO: this address can be returned at post_buy_orders, the trading service does not need it yet!
        exchange_success_address: bitcoin_rpc::Address::from("mtgyGsXBNG7Yta5rcMgWH4x9oGE5rm3ty9"),
    };

    match event_store.store_offer(offer_event.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            // TODO: create a to_string for e to return something nice.
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
    pub short_relative_time_lock: EthTimestamp,
}

impl From<TradeEvent> for TradeRequestResponse {
    fn from(trade_event: TradeEvent) -> Self {
        TradeRequestResponse {
            uid: trade_event.uid.clone(),
            exchange_refund_address: trade_event.exchange_refund_address.clone(),
            short_relative_time_lock: trade_event.short_relative_time_lock.clone(),
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
            error!("{:?}", e);
            return Err(BadRequest(Some(e.to_string())));
        }
    };

    // TODO: need to lock on uid now.

    // TODO: retrieve and use real address
    // This should never be used. Private key is: '9774cd25996588ef4bace0984eac1a80a3897c0cd3eea9858a6063c74f59e08b'
    let exchange_refund_address =
        EthAddress("0x1084d2C416fcc39564a4700a9B231270d463C5eA".to_string());

    let trade_event = TradeEvent {
        uid,
        secret_hash: trade_request_body.secret_hash,
        client_refund_address: trade_request_body.client_refund_address,
        long_relative_time_lock: trade_request_body.long_relative_time_lock,
        short_relative_time_lock: EthTimestamp(12), //TODO: this is obviously not "12" :)
        client_success_address: trade_request_body.client_success_address,
        exchange_refund_address: exchange_refund_address.clone(),
    };

    match event_store.store_trade(trade_event.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            // TODO: create a to_string for e to return something nice.
            return Err(BadRequest(None));
        }
    }

    Ok(Json(TradeRequestResponse::from(trade_event)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::{Client, LocalResponse};
    use rocket_factory::create_rocket_instance;
    use serde_json;
    use types;

    fn request_offer(client: &mut Client) -> LocalResponse {
        let offer_request = OfferRequestBody { amount: 42 };

        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(serde_json::to_string(&offer_request).unwrap());
        request.dispatch()
    }

    fn request_trade(client: &mut Client, uid: Uuid) -> LocalResponse {
        let trade_request = TradeRequestBody {
            secret_hash: SecretHash("MySecretHash".to_string()),
            client_refund_address: bitcoin_rpc::Address::from("ClientRefundAddressInBtc"),
            client_success_address: EthAddress("0xClientSuccessAddressInEth".to_string()),
            long_relative_time_lock: BtcBlockHeight(24),
        };

        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            .body(serde_json::to_string(&trade_request).unwrap());
        request.dispatch()
    }

    #[test]
    fn given_a_buy_offer_and_trade_should_respond_with_ok() {
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
                (trade_response.short_relative_time_lock > types::EthTimestamp(0)),
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
    fn given_two_trade_request_with_same_uid_should_fail() {
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
                (trade_response.short_relative_time_lock > types::EthTimestamp(0)),
                "Expected to receive a time-lock in response of trade_offer. Json Response:\n{:?}",
                trade_response
            );
        }

        {
            let mut response = request_trade(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }
}
