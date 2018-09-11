use bitcoin_support::{self, BitcoinQuantity};
use comit_node_api_client::{ApiClient, OrderRequestBody};
use common_types::{seconds::Seconds, secret::Secret, TradingSymbol};
use ethereum_htlc;
use ethereum_support::{self, EthereumQuantity};
use event_store::{EventStore, InMemoryEventStore};
use ganp::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
use rand::OsRng;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::{Arc, Mutex};
use swaps::{
    alice_events::{OfferCreated, OrderCreated, OrderTaken},
    common::TradeId,
    errors::Error,
};

#[derive(Deserialize)]
pub struct SellOfferRequestBody {
    amount: f64,
}

#[derive(Deserialize)]
pub struct SellOrderRequestBody {
    alice_success_address: bitcoin_support::Address,
    alice_refund_address: ethereum_support::Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    address_to_fund: ethereum_support::Address,
    btc_amount: BitcoinQuantity,
    eth_amount: EthereumQuantity,
    data: ethereum_htlc::ByteCode,
    gas: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OfferResponseBody<Buy: Ledger, Sell: Ledger> {
    //TODO use some kind of common types
    pub uid: TradeId,
    pub symbol: TradingSymbol,
    pub rate: f64,
    pub buy_amount: Buy::Quantity,
    pub sell_amount: Sell::Quantity,
}

impl<Buy: Ledger, Sell: Ledger> From<OfferResponseBody<Buy, Sell>> for OfferCreated<Buy, Sell> {
    fn from(offer: OfferResponseBody<Buy, Sell>) -> Self {
        OfferCreated {
            uid: offer.uid,
            symbol: offer.symbol,
            rate: offer.rate,
            buy_amount: offer.buy_amount,
            sell_amount: offer.sell_amount,
        }
    }
}

const ETH_HTLC_TIMEOUT_IN_SECONDS: Seconds = Seconds::new(12 * 60 * 60);

#[post("/trades/ETH-BTC/sell-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_sell_offers(
    offer_request_body: Json<SellOfferRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody<Bitcoin, Ethereum>>, BadRequest<String>> {
    let symbol = TradingSymbol::ETH_BTC;

    let offer_response_body =
        handle_sell_offer(event_store.inner(), offer_request_body.into_inner(), symbol)?;

    Ok(Json(offer_response_body))
}

fn handle_sell_offer(
    event_store: &InMemoryEventStore<TradeId>,
    offer_request_body: SellOfferRequestBody,
    symbol: TradingSymbol,
) -> Result<OfferResponseBody<Bitcoin, Ethereum>, Error> {
    let rate = 0.1; //TODO export this somewhere
    let sell_amount = offer_request_body.amount;
    let buy_amount = sell_amount * rate;

    let offer: OfferResponseBody<Bitcoin, Ethereum> = OfferResponseBody {
        uid: Default::default(),
        symbol,
        rate,
        sell_amount: ethereum_support::EthereumQuantity::from_eth(sell_amount),
        buy_amount: bitcoin_support::BitcoinQuantity::from_bitcoin(buy_amount),
    };
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
    let alice_success_address = sell_order.alice_success_address;
    let alice_refund_address = sell_order.alice_refund_address;

    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };

    //TODO: Remove before prod
    debug!("Secret: {:x}", secret);

    let lock_duration = ETH_HTLC_TIMEOUT_IN_SECONDS;

    let order_created_event: OrderCreated<Bitcoin, Ethereum> = OrderCreated {
        uid: trade_id,
        secret: secret.clone(),
        alice_success_address: alice_success_address.clone(),
        alice_refund_address: alice_refund_address.clone(),
        long_relative_timelock: lock_duration,
    };

    event_store.add_event(trade_id, order_created_event.clone())?;

    let order_response = client
        .create_sell_order(
            offer.symbol,
            trade_id,
            &OrderRequestBody {
                contract_secret_lock: secret.hash(),
                alice_refund_address: alice_refund_address,
                alice_success_address: alice_success_address,
                alice_contract_time_lock: lock_duration,
                buy_amount: offer.buy_amount,
                sell_amount: offer.sell_amount,
            },
        )
        .map_err(Error::ComitNode)?;

    let htlc = ethereum_htlc::EtherHtlc::new(
        lock_duration.into(),
        alice_refund_address,
        order_response.bob_success_address,
        secret.hash(),
    );

    let byte_code = htlc.compile_to_hex();

    let order_taken_event: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        bob_contract_time_lock: order_response.bob_contract_time_lock,
        bob_refund_address: order_response.bob_refund_address,
        bob_success_address: order_response.bob_success_address,
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
