use bitcoin_rpc;
use event_store::EventStore;
use event_store::OfferCreated;
pub use event_store::OfferCreated as OfferRequestResponse;
use event_store::OfferState;
use event_store::OrderTaken;
use rocket::State;
use rocket::http::RawStr;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use rocket_factory::TreasuryApiUrl;
use std::time::UNIX_EPOCH;
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
pub struct OrderRequestBody {
    pub contract_secret_lock: eth_htlc::SecretHash,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,

    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: eth_htlc::Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTakenResponseBody {
    pub exchange_refund_address: eth_htlc::Address,
    pub exchange_success_address: bitcoin_rpc::Address,
    pub exchange_contract_time_lock: u64,
}

impl From<OrderTaken> for OrderTakenResponseBody {
    fn from(order_taken_event: OrderTaken) -> Self {
        OrderTakenResponseBody {
            exchange_refund_address: order_taken_event.exchange_refund_address(),
            exchange_success_address: order_taken_event.exchange_success_address(),
            exchange_contract_time_lock: order_taken_event
                .exchange_contract_time_lock()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[post("/trades/ETH-BTC/<trade_id>/buy-orders", format = "application/json",
       data = "<order_request_body>")]
pub fn post_buy_orders(
    trade_id: &RawStr,
    order_request_body: Json<OrderRequestBody>,
    event_store: State<EventStore>,
) -> Result<Json<OrderTakenResponseBody>, BadRequest<String>> {
    // Receive trade information
    // - Hashed Secret
    // - Client refund address (BTC)
    // - timeout (BTC)
    // - Client success address (ETH)
    // = generates exchange refund address
    // -> returns ETH HTLC data (exchange refund address + ETH timeout)

    let order_request_body: OrderRequestBody = order_request_body.into_inner();
    let uid = match Uuid::parse_str(trade_id.as_str()) {
        Ok(uid) => uid,
        Err(e) => {
            error!("{}", e);
            return Err(BadRequest(Some(format!("Error when parsing uid: {}", e))));
        }
    };

    let order_taken = OrderTaken::new(
        uid,
        order_request_body.contract_secret_lock,
        order_request_body.client_contract_time_lock,
        order_request_body.client_refund_address,
        order_request_body.client_success_address,
        "1084d2C416fcc39564a4700a9B231270d463C5eA".into_address(),
        // TODO: retrieve and use real address
        // This should never be used. Private key is: 'cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8'
        bitcoin_rpc::Address::from(
            "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
        ),
    );

    match event_store.store_order_taken(order_taken.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            // TODO: create a to_string for e to return something nice.
            return Err(BadRequest(Some(e.to_string())));
        }
    }

    Ok(Json(offer.into()))
}

#[post("/trades/ETH-BTC/<trade_id>/buy_orders/fundings", format = "application/json")]
pub fn post_buy_orders_fundings(
    trade_id: &RawStr,
    event_store: State<EventStore>,
) -> Result<(), BadRequest<String>> {
    // Notification about received funds

    let htlc = eth_htlc::Htlc::new(
        order_taken.exchange_contract_time_lock(),
        order_taken.exchange_refund_address(),
        order_taken.client_refund_address(),
        order_taken.contract_secret_lock(),
    );

    // get contract bytecode
    // build creation transaction
    // sign transaction
    // send contract to blockchain
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

    fn request_order(client: &mut Client, uid: Uuid) -> LocalResponse {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "contract_secret_lock": "0x68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec",
                    "client_refund_address": "ClientRefundAddressInBtc",
                    "client_success_address": "0x956abb53d3ccbf24cf2f8c6e334a56d4b6c50440",
                    "client_contract_time_lock": 24
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
            let mut response = request_order(&mut client, uid);
            assert_eq!(response.status(), Status::Ok);

            #[derive(Deserialize)]
            struct Response {
                exchange_contract_time_lock: i64,
            }

            serde_json::from_str::<Response>(&response.body_string().unwrap()).unwrap();
        }
    }

    #[test]
    fn given_a_order_request_without_offer_should_fail() {
        let url = TreasuryApiUrl("stub".to_string());
        let event_store = EventStore::new();

        let rocket = create_rocket_instance(url, event_store);
        let mut client = rocket::local::Client::new(rocket).unwrap();

        let uid = Uuid::new_v4();

        {
            let response = request_order(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }

    #[test]
    fn given_two_orders_request_with_same_uid_should_fail() {
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
            let response = request_order(&mut client, uid);
            assert_eq!(response.status(), Status::Ok);
        }

        {
            let response = request_order(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }
}
