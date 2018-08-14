pub use super::events::OfferCreated as OfferRequestResponse;
use super::events::{ContractDeployed, OfferCreated, OfferState, OrderTaken, TradeFunded};
use bitcoin_fee_service::{self, BitcoinFeeService};
use bitcoin_htlc::{self, UnlockingError};
use bitcoin_rpc;
use bitcoin_support::{self, Network, PubkeyHash, ToP2wpkhAddress};
use bitcoin_witness::{PrimedInput, PrimedTransaction};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    secret::{Secret, SecretHash},
    TradingSymbol,
};
use ethereum_htlc;
use ethereum_service;
use ethereum_support;
use event_store::{self, EventStore, InMemoryEventStore};
use reqwest;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use secp256k1_support::KeyPair;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use swaps::TradeId;
use treasury_api_client::ApiClient;

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    TreasuryService(reqwest::Error),
    FeeService(bitcoin_fee_service::Error),
    EthereumService(ethereum_service::Error),
    BitcoinRpc(bitcoin_rpc::RpcError),
    BitcoinNode(reqwest::Error),
    Unlocking(String),
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

impl From<bitcoin_fee_service::Error> for Error {
    fn from(e: bitcoin_fee_service::Error) -> Self {
        Error::FeeService(e)
    }
}

impl From<bitcoin_rpc::RpcError> for Error {
    fn from(e: bitcoin_rpc::RpcError) -> Self {
        Error::BitcoinRpc(e)
    }
}

impl From<ethereum_service::Error> for Error {
    fn from(e: ethereum_service::Error) -> Self {
        Error::EthereumService(e)
    }
}

impl From<UnlockingError> for Error {
    fn from(e: UnlockingError) -> Self {
        match e {
            UnlockingError::WrongSecret { .. } => {
                Error::Unlocking(format!("{:?}", e).to_string())
            }
            UnlockingError::WrongKeyPair { .. } => {
                Error::Unlocking("exchange_success_public_key_hash was inconsistent with exchange_success_private_key".to_string())
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferRequestBody {
    pub amount: f64,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<OfferRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    treasury_api_client: State<Arc<ApiClient>>,
) -> Result<Json<OfferState<Ethereum, Bitcoin>>, BadRequest<String>> {
    let offer_state = handle_post_buy_offers(
        offer_request_body.into_inner(),
        event_store.inner(),
        treasury_api_client.inner(),
    )?;

    Ok(Json(offer_state)) // offer_event is the same than state.
}

fn handle_post_buy_offers(
    offer_request_body: OfferRequestBody,
    event_store: &InMemoryEventStore<TradeId>,
    treasury_api_client: &Arc<ApiClient>,
) -> Result<OfferState<Ethereum, Bitcoin>, Error> {
    let rate_response_body = treasury_api_client
        .request_rate(TradingSymbol::ETH_BTC, offer_request_body.amount)
        .map_err(Error::TreasuryService)?;

    let offer_event = OfferCreated::from(rate_response_body);

    event_store.add_event(offer_event.uid, offer_event.clone())?;

    info!("Created new offer: {:?}", offer_event);

    Ok(offer_event)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody {
    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: bitcoin_rpc::BlockHeight,

    pub client_refund_address: bitcoin_rpc::Address,
    pub client_success_address: ethereum_support::Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTakenResponseBody {
    pub exchange_refund_address: ethereum_support::Address,
    pub exchange_success_address: bitcoin_rpc::Address,
    pub exchange_contract_time_lock: u64,
}

impl From<OrderTaken> for OrderTakenResponseBody {
    fn from(order_taken_event: OrderTaken) -> Self {
        OrderTakenResponseBody {
            exchange_refund_address: order_taken_event.exchange_refund_address.into(),
            exchange_success_address: order_taken_event.exchange_success_address.into(),
            exchange_contract_time_lock: order_taken_event
                .exchange_contract_time_lock
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-orders",
    format = "application/json",
    data = "<order_request_body>"
)]
pub fn post_buy_orders(
    trade_id: TradeId,
    order_request_body: Json<OrderRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    exchange_success_keypair: State<KeyPair>,
    exchange_refund_address: State<ethereum_support::Address>,
    network: State<Network>,
) -> Result<Json<OrderTakenResponseBody>, BadRequest<String>> {
    let order_taken_response_body = handle_post_buy_orders(
        trade_id,
        order_request_body.into_inner(),
        event_store.inner(),
        exchange_success_keypair.inner(),
        exchange_refund_address.inner(),
        network.inner(),
    )?;
    Ok(Json(order_taken_response_body))
}

fn handle_post_buy_orders(
    trade_id: TradeId,
    order_request_body: OrderRequestBody,
    event_store: &InMemoryEventStore<TradeId>,
    exchange_success_keypair: &KeyPair,
    exchange_refund_address: &ethereum_support::Address,
    network: &Network,
) -> Result<OrderTakenResponseBody, Error> {
    // Receive trade information
    // - Hashed Secret
    // - Client refund address (BTC)
    // - timeout (BTC)
    // - Client success address (ETH)
    // = generates exchange refund address
    // -> returns ETH HTLC data (exchange refund address + ETH timeout)
    let client_refund_address: bitcoin_support::Address =
        order_request_body.client_refund_address.into();

    let twelve_hours = Duration::new(60 * 60 * 12, 0);

    let order_taken = OrderTaken {
        uid: trade_id,
        contract_secret_lock: order_request_body.contract_secret_lock,
        client_contract_time_lock: order_request_body.client_contract_time_lock,
        exchange_contract_time_lock: SystemTime::now() + twelve_hours,
        client_refund_address,
        client_success_address: order_request_body.client_success_address,
        exchange_refund_address: *exchange_refund_address,
        exchange_success_address: exchange_success_keypair
            .public_key()
            .clone()
            .to_p2wpkh_address(*network),
        exchange_success_keypair: exchange_success_keypair.clone(),
    };

    event_store.add_event(trade_id, order_taken.clone())?;
    Ok(order_taken.into())
}

#[derive(Deserialize)]
pub struct BuyOrderHtlcFundedNotification {
    transaction_id: bitcoin_rpc::TransactionId,
    vout: u32,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-htlc-funded",
    format = "application/json",
    data = "<buy_order_htlc_funded_notification>"
)]
pub fn post_buy_orders_fundings(
    trade_id: TradeId,
    buy_order_htlc_funded_notification: Json<BuyOrderHtlcFundedNotification>,
    event_store: State<InMemoryEventStore<TradeId>>,
    ethereum_service: State<Arc<ethereum_service::EthereumService>>,
) -> Result<(), BadRequest<String>> {
    handle_post_buy_order_funding(
        trade_id,
        buy_order_htlc_funded_notification.into_inner(),
        event_store.inner(),
        ethereum_service.inner(),
    )?;
    Ok(())
}

fn handle_post_buy_order_funding(
    trade_id: TradeId,
    buy_order_htlc_funded_notification: BuyOrderHtlcFundedNotification,
    event_store: &InMemoryEventStore<TradeId>,
    ethereum_service: &Arc<ethereum_service::EthereumService>,
) -> Result<(), Error> {
    let trade_funded = TradeFunded {
        uid: trade_id,
        transaction_id: buy_order_htlc_funded_notification.transaction_id.clone(),
        vout: buy_order_htlc_funded_notification.vout,
    };

    event_store.add_event(trade_id.clone(), trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken>(trade_id.clone())?;

    let htlc = ethereum_htlc::Htlc::new(
        order_taken.exchange_contract_time_lock,
        order_taken.exchange_refund_address,
        order_taken.client_success_address,
        order_taken.contract_secret_lock.clone(),
    );

    let offer_created_event =
        event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id.clone())?;

    let htlc_funding = offer_created_event.buy_amount.wei();

    let tx_id = ethereum_service.deploy_htlc(htlc, htlc_funding)?;

    event_store.add_event(
        trade_id,
        ContractDeployed {
            uid: trade_id,
            transaction_id: tx_id,
        },
    )?;

    Ok(())
}

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    pub secret: Secret,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-secret-revealed",
    format = "application/json",
    data = "<redeem_btc_notification_body>"
)]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    rpc_client: State<Arc<bitcoin_rpc::BitcoinRpcApi>>,
    fee_service: State<Arc<BitcoinFeeService>>,
    btc_exchange_redeem_address: State<bitcoin_support::Address>,
    trade_id: TradeId,
) -> Result<(), BadRequest<String>> {
    handle_post_revealed_secret(
        redeem_btc_notification_body.into_inner(),
        event_store.inner(),
        rpc_client.inner(),
        fee_service.inner(),
        btc_exchange_redeem_address.inner(),
        trade_id,
    )?;

    Ok(())
}

fn handle_post_revealed_secret(
    redeem_btc_notification_body: RedeemBTCNotificationBody,
    event_store: &InMemoryEventStore<TradeId>,
    rpc_client: &Arc<bitcoin_rpc::BitcoinRpcApi>,
    fee_service: &Arc<BitcoinFeeService>,
    btc_exchange_redeem_address: &bitcoin_support::Address,
    trade_id: TradeId,
) -> Result<(), Error> {
    let order_taken_event = event_store.get_event::<OrderTaken>(trade_id.clone())?;
    let offer_created_event =
        event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id.clone())?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event = event_store.get_event::<TradeFunded>(trade_id.clone())?;
    let secret: Secret = redeem_btc_notification_body.secret;
    let exchange_success_address = order_taken_event.exchange_success_address;
    let exchange_success_pubkey_hash: PubkeyHash = exchange_success_address.into();
    let exchange_success_keypair = order_taken_event.exchange_success_keypair;

    let client_refund_pubkey_hash: PubkeyHash = order_taken_event.client_refund_address.into();
    let htlc_txid = trade_funded_event.transaction_id;
    let vout = trade_funded_event.vout;

    let htlc = bitcoin_htlc::Htlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        order_taken_event.contract_secret_lock.clone(),
        order_taken_event.client_contract_time_lock.clone().into(),
    );

    htlc.can_be_unlocked_with(&secret, &exchange_success_keypair)?;

    let unlocking_parameters = htlc.unlock_with_secret(exchange_success_keypair.clone(), secret);

    let primed_txn = PrimedTransaction {
        inputs: vec![PrimedInput::new(
            htlc_txid.clone().into(),
            vout,
            offer_created_event.sell_amount,
            unlocking_parameters,
        )],
        output_address: btc_exchange_redeem_address.clone(),
        locktime: 0,
    };

    let total_input_value = primed_txn.total_input_value();

    let rate = fee_service.get_recommended_fee()?;
    let redeem_tx = primed_txn.sign_with_rate(rate);

    debug!(
        "Redeem {} (input: {}, vout: {}) to {} (output: {})",
        htlc_txid,
        total_input_value,
        vout,
        redeem_tx.txid(),
        redeem_tx.output[0].value
    );
    //TODO: Store above in event prior to doing rnpc request
    let rpc_transaction = bitcoin_rpc::SerializedRawTransaction::from(redeem_tx);
    debug!("RPC Transaction: {:?}", rpc_transaction);
    info!(
        "Attempting to redeem HTLC with txid {} for {}",
        htlc_txid, trade_id
    );
    //TODO: Store successful redeem in event
    let redeem_txid = rpc_client
        .send_raw_transaction(rpc_transaction)
        .map_err(Error::BitcoinNode)?
        .into_result()?;

    info!(
        "HTLC for {} successfully redeemed with {}",
        trade_id, redeem_txid
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_fee_service::StaticBitcoinFeeService;
    use bitcoin_support;
    use ethereum_service::BlockingEthereumApi;
    use ethereum_support::*;
    use ethereum_wallet::fake::StaticFakeWallet;
    use gas_price_service::StaticGasPriceService;
    use rocket::{
        self,
        http::{ContentType, Status},
        local::{Client, LocalResponse},
    };
    use rocket_factory::create_rocket_instance;
    use serde::Deserialize;
    use serde_json;
    use std::{str::FromStr, sync::Arc};
    use treasury_api_client::FakeApiClient;

    extern crate env_logger;

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
                    "client_refund_address": "bcrt1qcqslz7lfn34dl096t5uwurff9spen5h4v2pmap",
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
            InMemoryEventStore::new(),
            Arc::new(ethereum_service::EthereumService::new(
                Arc::new(StaticFakeWallet::account0()),
                Arc::new(StaticGasPriceService::default()),
                Arc::new(StaticEthereumApi),
                0,
            )),
            Arc::new(bitcoin_rpc::BitcoinStubClient::new()),
            "e7b6bfabddfaeb2c016b334a5322e4327dc5e499".into(),
            bitcoin_support::PrivateKey::from_str(
                "cR6U4gNiCQsPo5gLNP2w6QsLTZkvCGEijhYVPZVhnePQKjMwmas8",
            ).unwrap()
                .secret_key()
                .clone()
                .into(),
            bitcoin_support::Address::from_str("2NBNQWga7p2yEZmk1m5WuMxK5SyXM5cBZSL").unwrap(),
            Network::BitcoinCoreRegtest,
            Arc::new(StaticBitcoinFeeService::new(50.0)),
        );
        rocket::local::Client::new(rocket).unwrap()
    }

    #[test]
    fn given_an_offer_request_then_return_valid_offer_response() {
        let _ = env_logger::try_init();

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
        let _ = env_logger::try_init();

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
        let _ = env_logger::try_init();

        let mut client = create_rocket_client();

        let uid = "d9ee2df7-c330-4893-8345-6ba171f96e8f";

        {
            let response = request_order(&mut client, uid);
            assert_eq!(response.status(), Status::BadRequest);
        }
    }

    #[test]
    fn given_two_orders_request_with_same_uid_should_fail() {
        let _ = env_logger::try_init();

        let mut client = create_rocket_client();

        let uid = {
            let mut response = request_offer(&mut client);
            assert_eq!(response.status(), Status::Ok);

            let response = serde_json::from_str::<serde_json::Value>(
                &response.body_string().unwrap(),
            ).unwrap();

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
        let _ = env_logger::try_init();

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
