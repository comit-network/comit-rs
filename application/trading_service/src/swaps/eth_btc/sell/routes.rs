use bitcoin_support::{self, BitcoinQuantity, Network};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    secret::Secret,
    TradingSymbol,
};
use ethereum_htlc;
use ethereum_support::{self, Bytes, EthereumQuantity};
use event_store::{self, EventStore, InMemoryEventStore};
use exchange_api_client::{ApiClient, OfferResponseBody, OrderRequestBody};
use rand::OsRng;
use reqwest;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use rustc_hex;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use swaps::{
    events::{OfferCreated, OrderCreated, OrderTaken},
    TradeId,
};

#[derive(Debug)]
pub enum Error {
    EventStore(event_store::Error),
    ExchangeService(reqwest::Error),
    TradingService(String),
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

impl From<bitcoin_support::Error> for Error {
    fn from(_e: bitcoin_support::Error) -> Self {
        error!("{}", _e);
        Error::TradingService(String::from("Invalid bitcoin address format"))
    }
}

impl From<rustc_hex::FromHexError> for Error {
    fn from(_e: rustc_hex::FromHexError) -> Self {
        error!("{}", _e);
        Error::TradingService(String::from("Invalid ethereum address format"))
    }
}

#[derive(Deserialize)]
pub struct SellOfferRequestBody {
    amount: f64,
}

#[derive(Deserialize)]
pub struct SellOrderRequestBody {
    client_success_address: bitcoin_support::Address,
    client_refund_address: ethereum_support::Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    address_to_fund: ethereum_support::Address,
    btc_amount: BitcoinQuantity,
    eth_amount: EthereumQuantity,
    data: ethereum_htlc::ByteCode,
    gas: u64,
}

const ETH_HTLC_TIMEOUT: Duration = Duration::from_secs(12 * 60 * 60); //ethereum HTLC timout in seconds

#[post("/trades/ETH-BTC/sell-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_sell_offers(
    offer_request_body: Json<SellOfferRequestBody>,
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let symbol = TradingSymbol::ETH_BTC;

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
    offer_request_body: SellOfferRequestBody,
    symbol: TradingSymbol,
) -> Result<OfferResponseBody, Error> {
    let offer = client
        .create_sell_offer(symbol, offer_request_body.amount)
        .map_err(Error::ExchangeService)?;
    let id = offer.uid.clone();
    let event: OfferCreated<Bitcoin, Ethereum> = OfferCreated::from(offer.clone());
    event_store.add_event(id, event)?;
    Ok(offer)
}

#[post(
    "/trades/ETH-BTC/<trade_id>/sell-orders",
    format = "application/json",
    data = "<sell_order_request_body>"
)]
pub fn post_sell_orders(
    trade_id: TradeId,
    sell_order_request_body: Json<SellOrderRequestBody>,
    client: State<Arc<ApiClient>>,
    _network: State<Network>,
    event_store: State<InMemoryEventStore<TradeId>>,
    rng: State<Mutex<OsRng>>,
) -> Result<Json<RequestToFund>, BadRequest<String>> {
    let request_to_fund = handle_sell_orders(
        client.inner(),
        event_store.inner(),
        rng.inner(),
        trade_id,
        sell_order_request_body.into_inner(),
    )?;

    Ok(Json(request_to_fund))
}

fn handle_sell_orders(
    client: &Arc<ApiClient>,
    event_store: &InMemoryEventStore<TradeId>,
    rng: &Mutex<OsRng>,
    trade_id: TradeId,
    sell_order: SellOrderRequestBody,
) -> Result<RequestToFund, Error> {
    let offer: OfferCreated<Bitcoin, Ethereum> = event_store.get_event(trade_id)?;
    let client_success_address = sell_order.client_success_address;
    let client_refund_address = sell_order.client_refund_address;

    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };

    //TODO: Remove before prod
    debug!("Secret: {:x}", secret);

    let order_created_event: OrderCreated<Bitcoin, Ethereum> = OrderCreated {
        uid: trade_id,
        secret: secret.clone(),
        client_success_address: client_success_address.clone(),
        client_refund_address: client_refund_address.clone(),
        long_relative_timelock: ETH_HTLC_TIMEOUT,
    };

    event_store.add_event(trade_id, order_created_event.clone())?;

    let order_response = client
        .create_sell_order(
            offer.symbol,
            trade_id,
            &OrderRequestBody {
                contract_secret_lock: secret.hash(),
                client_refund_address: client_refund_address.to_string(),
                client_success_address: client_success_address.to_string(),
                client_contract_time_lock: ETH_HTLC_TIMEOUT.as_secs(),
            },
        )
        .map_err(Error::ExchangeService)?;

    let htlc = ethereum_htlc::Htlc::new(
        ETH_HTLC_TIMEOUT,
        client_refund_address,
        order_response.exchange_success_address.parse()?,
        secret.hash(),
    );

    let byte_code = htlc.compile_to_hex();
    let bytes: Bytes = htlc.compile_to_hex().into();

    let order_taken_event: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        exchange_contract_time_lock: order_response.exchange_contract_time_lock,
        exchange_refund_address: order_response.exchange_refund_address.parse()?,
        exchange_success_address: order_response.exchange_success_address.parse()?,
        htlc: bytes.0,
    };

    event_store.add_event(trade_id, order_taken_event)?;

    let offer = event_store.get_event::<OfferCreated<Bitcoin, Ethereum>>(trade_id)?;

    let fund = RequestToFund {
        address_to_fund: "0000000000000000000000000000000000000000".parse()?,
        btc_amount: offer.buy_amount,
        eth_amount: offer.sell_amount,
        data: byte_code,
        gas: 21_000u64,
    };
    Ok(fund)
}
