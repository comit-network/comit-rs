use bitcoin_htlc::{self, Htlc as BtcHtlc};
use bitcoin_rpc::BlockHeight;
use bitcoin_support::{self, BitcoinQuantity, Network, PubkeyHash};
use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    TradingSymbol,
};
use ethereum_support::{self, EthereumQuantity};
use event_store::{EventStore, InMemoryEventStore};
use exchange_api_client::{ApiClient, OfferResponseBody, OrderRequestBody};
use rand::OsRng;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use secret::Secret;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use swaps::{
    errors::Error,
    events::{ContractDeployed, OfferCreated, OrderCreated, OrderTaken},
    TradeId,
};

#[derive(Deserialize)]
pub struct BuyOfferRequestBody {
    amount: f64,
}

#[post("/trades/ETH-BTC/buy-offers", format = "application/json", data = "<offer_request_body>")]
pub fn post_buy_offers(
    offer_request_body: Json<BuyOfferRequestBody>,
    client: State<Arc<ApiClient>>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<Json<OfferResponseBody>, BadRequest<String>> {
    let offer_request_body = offer_request_body.into_inner();
    let symbol = TradingSymbol::ETH_BTC;

    let res = client.create_buy_offer(symbol, offer_request_body.amount);
    let offer_response = res.map_err(Error::ExchangeService)?;
    let id = offer_response.uid.clone();
    let event: OfferCreated<Ethereum, Bitcoin> = OfferCreated::from(offer_response.clone());

    event_store.add_event(id, event).map_err(Error::EventStore)?;
    Ok(Json(offer_response))
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

const BTC_BLOCKS_IN_24H: u32 = 24 * 60 / 10;

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

    let order_created_event: OrderCreated<Ethereum, Bitcoin> = OrderCreated {
        uid: trade_id,
        secret: secret.clone(),
        client_success_address: client_success_address.clone(),
        client_refund_address: client_refund_address.clone(),
        long_relative_timelock: BlockHeight::new(BTC_BLOCKS_IN_24H),
    };

    event_store.add_event(trade_id, order_created_event.clone())?;

    let order_response = client
        .create_buy_order(
            offer.symbol,
            trade_id,
            &OrderRequestBody {
                contract_secret_lock: secret.hash(),
                client_refund_address: client_refund_address.to_string(),
                client_success_address: client_success_address.to_string(),
                client_contract_time_lock: BTC_BLOCKS_IN_24H as u64,
            },
        )
        .map_err(Error::ExchangeService)?;

    let exchange_success_pubkey_hash = PubkeyHash::from(bitcoin_support::Address::from_str(
        order_response.exchange_success_address.as_str(),
    )?);
    let client_refund_pubkey_hash = PubkeyHash::from(client_refund_address);

    let htlc: BtcHtlc = BtcHtlc::new(
        exchange_success_pubkey_hash,
        client_refund_pubkey_hash,
        secret.hash(),
        BTC_BLOCKS_IN_24H,
    );

    let order_taken_event: OrderTaken<Ethereum, Bitcoin> = OrderTaken {
        uid: trade_id,
        exchange_contract_time_lock: Duration::from_secs(
            order_response.exchange_contract_time_lock,
        ),
        exchange_refund_address: order_response.exchange_refund_address.parse()?,
        exchange_success_address: order_response.exchange_success_address.parse()?,
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

#[derive(Deserialize)]
pub struct ContractDeployedRequestBody {
    pub contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-contract-deployed",
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
    address: ethereum_support::Address,
) -> Result<(), Error> {
    let deployed: ContractDeployed<Ethereum, Bitcoin> = ContractDeployed::new(uid, address);
    event_store.add_event(uid, deployed)?;

    Ok(())
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
