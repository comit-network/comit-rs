use bitcoin_htlc::{self, Htlc as BtcHtlc};
use bitcoin_support::{self, BitcoinQuantity, Blocks, Network, PubkeyHash};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    secret::Secret,
    TradingSymbol,
};
use ethereum_support::{self, EthereumQuantity};
use event_store::{EventStore, InMemoryEventStore};
use exchange_api_client::{ApiClient, OrderRequestBody};
use rand::OsRng;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::{Arc, Mutex};
use swaps::{
    alice_events::{ContractDeployed, OfferCreated, OrderCreated, OrderTaken},
    common::TradeId,
    errors::Error,
};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: f64,
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

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody<Ethereum, Bitcoin>>, BadRequest<String>> {
    let symbol = TradingSymbol::ETH_BTC;

    let offer = handle_buy_offer(event_store.inner(), offer_request_body.into_inner(), symbol)?;

    Ok(Json(offer))
}

fn handle_buy_offer(
    event_store: &InMemoryEventStore<TradeId>,
    offer_request_body: BuyOfferRequestBody,
    symbol: TradingSymbol,
) -> Result<OfferResponseBody<Ethereum, Bitcoin>, Error> {
    let rate = 0.1; //TODO export this somewhere
    let sell_amount = offer_request_body.amount;
    let buy_amount = sell_amount * rate;

    let offer: OfferResponseBody<Ethereum, Bitcoin> = OfferResponseBody {
        uid: TradeId::from(Uuid::new_v4()),
        symbol,
        rate,
        sell_amount: bitcoin_support::BitcoinQuantity::from_bitcoin(buy_amount),
        buy_amount: ethereum_support::EthereumQuantity::from_eth(sell_amount),
    };

    let id = offer.uid.clone();
    let event: OfferCreated<Ethereum, Bitcoin> = OfferCreated::from(offer.clone());

    event_store.add_event(id, event).map_err(Error::EventStore)?;
    Ok(offer)
}

#[derive(Deserialize)]
pub struct BuyOrderRequestBody {
    client_success_address: ethereum_support::Address,
    client_refund_address: bitcoin_support::Address,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestToFund {
    address_to_fund: bitcoin_support::Address,
    btc_amount: BitcoinQuantity,
    eth_amount: EthereumQuantity,
}

const BTC_BLOCKS_IN_24H: Blocks = Blocks::new(24 * 60 / 10);

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-orders",
    format = "application/json",
    data = "<buy_order_request_body>"
)]
pub fn post_buy_orders(
    trade_id: TradeId,
    buy_order_request_body: Json<BuyOrderRequestBody>,
    client: State<Arc<ApiClient>>,
    network: State<Network>,
    event_store: State<InMemoryEventStore<TradeId>>,
    rng: State<Mutex<OsRng>>,
) -> Result<Json<RequestToFund>, BadRequest<String>> {
    let request_to_fund = handle_buy_orders(
        client.inner(),
        event_store.inner(),
        rng.inner(),
        network.inner(),
        trade_id,
        buy_order_request_body.into_inner(),
    )?;

    Ok(Json(request_to_fund))
}

fn handle_buy_orders(
    client: &Arc<ApiClient>,
    event_store: &InMemoryEventStore<TradeId>,
    rng: &Mutex<OsRng>,
    network: &Network,
    trade_id: TradeId,
    buy_order: BuyOrderRequestBody,
) -> Result<RequestToFund, Error> {
    let offer = event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id)?;
    let client_success_address = buy_order.client_success_address;
    let client_refund_address = buy_order.client_refund_address;

    let secret = {
        let mut rng = rng.lock().unwrap();
        Secret::generate(&mut *rng)
    };

    //TODO: Remove before prod
    debug!("Secret: {:x}", secret);

    let lock_duration = BTC_BLOCKS_IN_24H;

    let order_created_event: OrderCreated<Ethereum, Bitcoin> = OrderCreated {
        uid: trade_id,
        secret: secret.clone(),
        client_success_address: client_success_address.clone(),
        client_refund_address: client_refund_address.clone(),
        long_relative_timelock: lock_duration.clone(),
    };

    event_store.add_event(trade_id, order_created_event.clone())?;

    let order_response = client
        .create_buy_order(
            offer.symbol,
            trade_id,
            &OrderRequestBody {
                contract_secret_lock: secret.hash(),
                client_refund_address: client_refund_address.clone(),
                client_success_address: client_success_address.clone(),
                client_contract_time_lock: lock_duration.clone(),
                buy_amount: offer.buy_amount,
                sell_amount: offer.sell_amount,
            },
        )
        .map_err(Error::ExchangeService)?;

    let exchange_success_pubkey_hash =
        PubkeyHash::from(order_response.exchange_success_address.clone());
    let client_refund_pubkey_hash = PubkeyHash::from(client_refund_address);

    let htlc: BtcHtlc = BtcHtlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        secret.hash(),
        lock_duration.into(),
    );

    let order_taken_event: OrderTaken<Ethereum, Bitcoin> = OrderTaken {
        uid: trade_id,
        exchange_contract_time_lock: order_response.exchange_contract_time_lock,
        exchange_refund_address: order_response.exchange_refund_address,
        exchange_success_address: order_response.exchange_success_address,
    };

    event_store.add_event(trade_id, order_taken_event)?;

    let offer = event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id)?;

    let htlc_address = htlc.compute_address(network.clone());

    Ok(RequestToFund {
        address_to_fund: htlc_address,
        eth_amount: offer.buy_amount,
        btc_amount: offer.sell_amount,
    })
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RedeemDetails {
    address: ethereum_support::Address,
    data: bitcoin_htlc::secret::Secret,
    gas: u64,
}

#[get("/trades/ETH-BTC/<trade_id>/redeem-orders")]
pub fn get_redeem_orders(
    trade_id: TradeId,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<RedeemDetails>, BadRequest<String>> {
    let details = handle_get_redeem_orders(event_store.inner(), trade_id)?;

    Ok(Json(details))
}

fn handle_get_redeem_orders(
    event_store: &InMemoryEventStore<TradeId>,
    trade_id: TradeId,
) -> Result<RedeemDetails, Error> {
    let address = event_store
        .get_event::<ContractDeployed<Ethereum, Bitcoin>>(trade_id)?
        .address;
    let secret = event_store
        .get_event::<OrderCreated<Ethereum, Bitcoin>>(trade_id)?
        .secret;

    Ok(RedeemDetails {
        address,
        data: secret,
        // TODO: check how much gas we should tell the customer to pay
        gas: 35000,
    })
}
