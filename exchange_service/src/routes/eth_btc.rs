use bitcoin_htlc::Network;
use bitcoin_rpc;
use bitcoin_wallet;
use bitcoin_wallet::ToP2wpkhAddress;
use common_types::secret::SecretHash;
use ethereum_htlc;
use ethereum_service;
use event_store;
use event_store::ContractDeployed;
use event_store::EventStore;
use event_store::OfferCreated;
pub use event_store::OfferCreated as OfferRequestResponse;
use event_store::OfferState;
use event_store::OrderTaken;
use event_store::TradeFunded;
use event_store::TradeId;
use rocket::State;
use rocket::http::RawStr;
use rocket::request::FromParam;
use rocket::response::status::BadRequest;
use rocket_contrib::Json;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use treasury_api_client::{ApiClient, Symbol};
use uuid;
use uuid::Uuid;
use web3::types::Address as EthereumAddress;

impl<'a> FromParam<'a> for TradeId {
    type Error = uuid::ParseError;

    fn from_param(param: &RawStr) -> Result<Self, <Self as FromParam>::Error> {
        Uuid::parse_str(param.as_str()).map(|uid| TradeId::from_uuid(uid))
    }
}

impl From<event_store::Error> for BadRequest<String> {
    fn from(e: event_store::Error) -> Self {
        error!("EventStore error: {:?}", e);
        BadRequest(None)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequestBody {
    pub amount: f64,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
fn post_buy_offers(
    offer_request_body: Json<OfferRequestBody>,
    event_store: State<EventStore>,
    treasury_api_client: State<Arc<ApiClient>>,
) -> Result<Json<OfferState>, BadRequest<String>> {
    let offer_request_body: OfferRequestBody = offer_request_body.into_inner();

    let res =
        treasury_api_client.request_rate(Symbol("ETH-BTC".to_string()), offer_request_body.amount);

    let rate_response_body = match res {
        Ok(rate) => rate,
        Err(e) => {
            error!("{:?}", e);
            return Err(BadRequest(None));
        }
    };

    let offer_event = OfferCreated::from(rate_response_body);

    match event_store.store_offer(offer_event.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{}", e);
            return Err(BadRequest(None));
        }
    }

    info!("Created new offer: {:?}", offer_event);

    Ok(Json(offer_event.clone())) // offer_event is the same than state.
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,

    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: EthereumAddress,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTakenResponseBody {
    pub exchange_refund_address: EthereumAddress,
    pub exchange_success_address: bitcoin_rpc::Address,
    pub exchange_contract_time_lock: u64,
}

impl From<OrderTaken> for OrderTakenResponseBody {
    fn from(order_taken_event: OrderTaken) -> Self {
        OrderTakenResponseBody {
            exchange_refund_address: order_taken_event.exchange_refund_address(),
            exchange_success_address: order_taken_event.exchange_success_address().into(),
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
    trade_id: TradeId,
    order_request_body: Json<OrderRequestBody>,
    event_store: State<EventStore>,
    exchange_success_private_key: State<bitcoin_wallet::PrivateKey>,
    exchange_refund_address: State<EthereumAddress>,
    network: State<Network>,
) -> Result<Json<OrderTakenResponseBody>, BadRequest<String>> {
    // Receive trade information
    // - Hashed Secret
    // - Client refund address (BTC)
    // - timeout (BTC)
    // - Client success address (ETH)
    // = generates exchange refund address
    // -> returns ETH HTLC data (exchange refund address + ETH timeout)

    let order_request_body: OrderRequestBody = order_request_body.into_inner();

    let order_taken = OrderTaken::new(
        trade_id,
        order_request_body.contract_secret_lock,
        order_request_body.client_contract_time_lock,
        order_request_body.client_refund_address,
        order_request_body.client_success_address,
        *exchange_refund_address,
        exchange_success_private_key.to_p2wpkh_address(*network),
        exchange_success_private_key.clone(),
    );

    match event_store.store_order_taken(order_taken.clone()) {
        Ok(_) => (),
        Err(e) => {
            error!("{:?}", e);
            // TODO: create a to_string for e to return something nice.
            return Err(BadRequest(Some(e.to_string())));
        }
    }

    Ok(Json(order_taken.into()))
}

#[derive(Deserialize)]
pub struct BuyOrderHtlcFundedNotification {
    transaction_id: bitcoin_rpc::TransactionId,
    vout: u32,
}

#[post("/trades/ETH-BTC/<trade_id>/buy-order-htlc-funded", format = "application/json",
       data = "<buy_order_htlc_funded_notification>")]
pub fn post_buy_orders_fundings(
    trade_id: TradeId,
    buy_order_htlc_funded_notification: Json<BuyOrderHtlcFundedNotification>,
    event_store: State<EventStore>,
    ethereum_service: State<Arc<ethereum_service::EthereumService>>,
) -> Result<(), BadRequest<String>> {
    let trade_funded = TradeFunded::new(
        trade_id,
        buy_order_htlc_funded_notification.transaction_id.clone(),
        buy_order_htlc_funded_notification.vout,
    );
    event_store.store_trade_funded(trade_funded)?;

    let order_taken = event_store.get_order_taken_event(&trade_id)?;

    let htlc = ethereum_htlc::Htlc::new(
        order_taken.exchange_contract_time_lock(),
        order_taken.exchange_refund_address(),
        order_taken.client_success_address(),
        order_taken.contract_secret_lock().clone(),
    );

    let offer_created_event = event_store.get_offer_created_event(&trade_id)?;

    let htlc_funding = offer_created_event.eth_amount().wei();

    let tx_id = match ethereum_service.deploy_htlc(htlc, htlc_funding) {
        Ok(tx_id) => tx_id,
        Err(e) => {
            // TODO: Should we rollback the TradeFunded event here?
            // We didn't successfully transition from TradeFunded to ContractDeployed.

            error!("Failed to deploy HTLC. Error: {:?}", e);
            return Err(BadRequest(None));
        }
    };

    event_store.store_contract_deployed(ContractDeployed::new(trade_id, tx_id))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_fee_service::StaticBitcoinFeeService;
    use bitcoin_htlc::Network;
    use common_types::BitcoinQuantity;
    use ethereum_service::BlockingEthereumApi;
    use ethereum_wallet::fake::StaticFakeWallet;
    use gas_price_service::StaticGasPriceService;
    use rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::{Client, LocalResponse};
    use rocket_factory::create_rocket_instance;
    use serde::Deserialize;
    use serde_json;
    use std::str::FromStr;
    use std::sync::Arc;
    use treasury_api_client::FakeApiClient;
    use web3;
    use web3::types::{Bytes, H256};

    fn request_offer(client: &mut Client) -> LocalResponse {
        let request = client
            .post("/trades/ETH-BTC/buy-offers")
            .header(ContentType::JSON)
            .body(r#"{ "amount": 42 }"#);
        request.dispatch()
    }

    fn request_order<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-orders", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "contract_secret_lock": "68d627971643a6f97f27c58957826fcba853ec2077fd10ec6b93d8e61deb4cec",
                    "client_refund_address": "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa",
                    "client_success_address": "0x956abb53d3ccbf24cf2f8c6e334a56d4b6c50440",
                    "client_contract_time_lock": 24
                  }"#,
            );
        request.dispatch()
    }

    fn notify_about_funding<'a>(client: &'a mut Client, uid: &str) -> LocalResponse<'a> {
        let request = client
            .post(format!("/trades/ETH-BTC/{}/buy-order-htlc-funded", uid).to_string())
            .header(ContentType::JSON)
            .body(
                r#"{
                    "transaction_id": "a02e9dc0ddc3d8200cc4be0e40a1573519a1a1e9b15e0c4c296fcaa65da80d43",
                    "vout" : 0
                  }"#,
            );
        request.dispatch()
    }

    trait DeserializeAsJson {
        fn body_json<T>(&mut self) -> T
        where
            for<'de> T: Deserialize<'de>;
    }

    impl<'r> DeserializeAsJson for LocalResponse<'r> {
        fn body_json<T>(&mut self) -> T
        where
            for<'de> T: Deserialize<'de>,
        {
            let body = self.body().unwrap().into_inner();

            serde_json::from_reader(body).unwrap()
        }
    }

    struct StaticEthereumApi;

    impl BlockingEthereumApi for StaticEthereumApi {
        fn send_raw_transaction(&self, _rlp: Bytes) -> Result<H256, web3::Error> {
            Ok(H256::new())
        }
    }

    fn create_rocket_client() -> Client {
        let rocket = create_rocket_instance(
            Arc::new(FakeApiClient),
            EventStore::new(),
            Arc::new(ethereum_service::EthereumService::new(
                Arc::new(StaticFakeWallet::account0()),
                Arc::new(StaticGasPriceService::default()),
                Arc::new(StaticEthereumApi),
                0,
            )),
            Arc::new(bitcoin_rpc::BitcoinStubClient::new()),
            "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
            bitcoin_wallet::PrivateKey::from_str(
                "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
            ).unwrap(),
            bitcoin_wallet::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap(),
            Network::BitcoinCoreRegtest,
            Arc::new(StaticBitcoinFeeService::new(BitcoinQuantity::from_satoshi(
                50,
            ))),
        );
        rocket::local::Client::new(rocket).unwrap()
    }

    #[test]
    fn given_an_offer_request_then_return_valid_offer_response() {
        let mut client = create_rocket_client();

        let mut response = request_offer(&mut client);
        assert_eq!(response.status(), Status::Ok);

        let offer_response = response.body_json::<serde_json::Value>();

        assert_eq!(
            offer_response["symbol"], "ETH-BTC",
            "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
            offer_response
        );
    }

    #[test]
    fn given_a_trade_request_when_buy_offer_was_done_then_return_valid_trade_response() {
        let mut client = create_rocket_client();

        let uid = {
            let mut response = request_offer(&mut client);
            assert_eq!(response.status(), Status::Ok);

            let offer_response = response.body_json::<serde_json::Value>();

            assert_eq!(
                offer_response["symbol"], "ETH-BTC",
                "Expected to receive a symbol in response of buy_offers. Json Response:\n{:?}",
                offer_response
            );

            offer_response["uid"].as_str().unwrap().to_string()
        };

        {
            let mut response = request_order(&mut client, &uid);
            assert_eq!(response.status(), Status::Ok);

            #[derive(Deserialize)]
            #[allow(dead_code)]
            struct Response {
                exchange_contract_time_lock: i64,
            }

            serde_json::from_str::<Response>(&response.body_string().unwrap()).unwrap();
        }
    }

    #[test]
    fn given_a_order_request_without_offer_should_fail() {
        let mut client = create_rocket_client();

        let uid = "d9ee2df7-c330-4893-8345-6ba171f96e8f";

        {
            let response = request_order(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }

    #[test]
    fn given_two_orders_request_with_same_uid_should_fail() {
        let mut client = create_rocket_client();

        let uid = {
            let mut response = request_offer(&mut client);
            assert_eq!(response.status(), Status::Ok);

            let response =
                serde_json::from_str::<serde_json::Value>(&response.body_string().unwrap())
                    .unwrap();

            response["uid"].as_str().unwrap().to_string()
        };

        {
            let response = request_order(&mut client, &uid);
            assert_eq!(response.status(), Status::Ok);
        }

        {
            let response = request_order(&mut client, &uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }

    #[test]
    fn given_an_accepted_trade_when_provided_with_funding_tx_should_deploy_htlc() {
        let mut client = create_rocket_client();

        let trade_id = {
            let mut response = request_offer(&mut client);

            assert_eq!(response.status(), Status::Ok);
            response.body_json::<serde_json::Value>()["uid"]
                .as_str()
                .unwrap()
                .to_string()
        };

        {
            let response = request_order(&mut client, &trade_id);
            assert_eq!(response.status(), Status::Ok)
        }

        {
            let response = notify_about_funding(&mut client, &trade_id);
            assert_eq!(response.status(), Status::Ok)
        }
    }
}
