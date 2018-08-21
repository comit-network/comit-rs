use bitcoin_rpc_client::BlockHeight;
use bitcoin_support::{self, BitcoinQuantity};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    secret::Secret,
    TradingSymbol,
};
use ethereum_htlc;
use ethereum_support::{self, EthereumQuantity};
use event_store::{EventStore, InMemoryEventStore};
use exchange_api_client::{ApiClient, OfferResponseBody, OrderRequestBody};
use rand::OsRng;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use swaps::{
    errors::Error,
    events::{ContractDeployed, OfferCreated, OrderCreated, OrderTaken},
    TradeId,
};

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

const ETH_HTLC_TIMEOUT_IN_SECONDS: Duration = Duration::from_secs(12 * 60 * 60);

#[post("/trades/ETH-BTC/sell-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_sell_offers(
    offer_request_body: Json<SellOfferRequestBody>,
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody<Bitcoin, Ethereum>>, BadRequest<String>> {
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
) -> Result<OfferResponseBody<Bitcoin, Ethereum>, Error> {
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
        long_relative_timelock: ETH_HTLC_TIMEOUT_IN_SECONDS,
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
                client_contract_time_lock: ETH_HTLC_TIMEOUT_IN_SECONDS.as_secs(),
            },
        )
        .map_err(Error::ExchangeService)?;

    let htlc = ethereum_htlc::Htlc::new(
        ETH_HTLC_TIMEOUT_IN_SECONDS,
        client_refund_address,
        order_response.exchange_success_address.parse()?,
        secret.hash(),
    );

    let byte_code = htlc.compile_to_hex();

    let order_taken_event: OrderTaken<Bitcoin, Ethereum> = OrderTaken {
        uid: trade_id,
        exchange_contract_time_lock: BlockHeight::new(
            order_response.exchange_contract_time_lock as u32,
        ),
        exchange_refund_address: order_response.exchange_refund_address.parse()?,
        exchange_success_address: order_response.exchange_success_address.parse()?,
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

#[derive(Deserialize)]
pub struct ContractDeployedRequestBody {
    pub contract_address: bitcoin_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/sell-order-contract-deployed",
    format = "application/json",
    data = "<contract_deployed_request_body>"
)]
pub fn post_contract_deployed(
    trade_id: TradeId,
    contract_deployed_request_body: Json<ContractDeployedRequestBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<(), BadRequest<String>> {
    handle_post_contract_deployed(
        event_store.inner(),
        trade_id,
        contract_deployed_request_body.into_inner().contract_address,
    )?;

    Ok(())
}

fn handle_post_contract_deployed(
    event_store: &InMemoryEventStore<TradeId>,
    uid: TradeId,
    address: bitcoin_support::Address,
) -> Result<(), Error> {
    let deployed: ContractDeployed<Bitcoin, Ethereum> = ContractDeployed::new(uid, address);
    event_store.add_event(uid, deployed)?;

    Ok(())
}

#[derive(Deserialize, Serialize, Debug)]
pub struct RedeemDetails {
    address: bitcoin_support::Address,
    data: Secret,
}

#[get("/trades/ETH-BTC/<trade_id>/redeem-sell-orders")]
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
        .get_event::<ContractDeployed<Bitcoin, Ethereum>>(trade_id)?
        .address;
    let secret = event_store
        .get_event::<OrderCreated<Bitcoin, Ethereum>>(trade_id)?
        .secret;

    Ok(RedeemDetails {
        address,
        data: secret,
    })
}
