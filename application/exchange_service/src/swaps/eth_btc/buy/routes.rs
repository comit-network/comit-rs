use bitcoin_fee_service;
use bitcoin_htlc::UnlockingError;
use bitcoin_rpc_client;
use bitcoin_service;
use bitcoin_support::{self, BitcoinQuantity, Network, ToP2wpkhAddress};
use common_types::{
    ledger::{
        bitcoin::{self, Bitcoin},
        ethereum::Ethereum,
        Ledger,
    },
    seconds::Seconds,
    secret::{Secret, SecretHash},
    TradingSymbol,
};
use ethereum_htlc;
use ethereum_service;
use ethereum_support;
use event_store::{self, EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swaps::{
    common::{Error, TradeId},
    events::{
        ContractDeployed, ContractRedeemed, OfferCreated as OfferState, OfferCreated, OrderTaken,
        TradeFunded,
    },
};
use treasury_api_client::ApiClient;

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

impl From<bitcoin_rpc_client::RpcError> for Error {
    fn from(e: bitcoin_rpc_client::RpcError) -> Self {
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
    let buy_amount = ethereum_support::EthereumQuantity::from_eth(offer_request_body.amount);

    let rate_response_body = treasury_api_client
        .request_rate(TradingSymbol::ETH_BTC)
        .map_err(Error::TreasuryService)?;
    let sell_amount =
        BitcoinQuantity::from_bitcoin(rate_response_body.rate * buy_amount.ethereum());

    let offer_event = OfferCreated::new(
        rate_response_body.rate,
        buy_amount,
        sell_amount,
        TradingSymbol::ETH_BTC,
    );

    event_store.add_event(offer_event.uid, offer_event.clone())?;

    info!("Created new offer: {:?}", offer_event);

    Ok(offer_event)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody<Buy: Ledger, Sell: Ledger> {
    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: Sell::LockDuration,
    pub client_refund_address: Sell::Address,
    pub client_success_address: Buy::Address,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTakenResponseBody<Buy: Ledger, Sell: Ledger> {
    pub exchange_refund_address: Buy::Address,
    pub exchange_success_address: Sell::Address,
    pub exchange_contract_time_lock: Buy::LockDuration,
}

impl<Buy: Ledger, Sell: Ledger> From<OrderTaken<Buy, Sell>> for OrderTakenResponseBody<Buy, Sell> {
    fn from(order_taken_event: OrderTaken<Buy, Sell>) -> Self {
        OrderTakenResponseBody {
            exchange_refund_address: order_taken_event.exchange_refund_address.into(),
            exchange_success_address: order_taken_event.exchange_success_address.into(),
            exchange_contract_time_lock: order_taken_event.exchange_contract_time_lock,
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
    order_request_body: Json<OrderRequestBody<Ethereum, Bitcoin>>,
    event_store: State<InMemoryEventStore<TradeId>>,
    exchange_success_keypair: State<KeyPair>,
    exchange_refund_address: State<ethereum_support::Address>,
    network: State<Network>,
) -> Result<Json<OrderTakenResponseBody<Ethereum, Bitcoin>>, BadRequest<String>> {
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
    order_request_body: OrderRequestBody<Ethereum, Bitcoin>,
    event_store: &InMemoryEventStore<TradeId>,
    exchange_success_keypair: &KeyPair,
    exchange_refund_address: &ethereum_support::Address,
    network: &Network,
) -> Result<OrderTakenResponseBody<Ethereum, Bitcoin>, Error> {
    // Receive trade information
    // - Hashed Secret
    // - Client refund address (BTC)
    // - timeout (BTC)
    // - Client success address (ETH)
    // = generates exchange refund address
    // -> returns ETH HTLC data (exchange refund address + ETH timeout)
    let client_refund_address: bitcoin_support::Address =
        order_request_body.client_refund_address.into();
    //TODO: clean up, should not need to do address>pub_key>address
    let exchange_success_address = bitcoin_support::Address::from(
        exchange_success_keypair
            .public_key()
            .clone()
            .to_p2wpkh_address(*network),
    );

    let twelve_hours = Seconds::new(60 * 60 * 12);

    let order_taken = OrderTaken {
        uid: trade_id,
        contract_secret_lock: order_request_body.contract_secret_lock,
        client_contract_time_lock: order_request_body.client_contract_time_lock,
        exchange_contract_time_lock: twelve_hours,
        client_refund_address,
        client_success_address: order_request_body.client_success_address,
        exchange_refund_address: *exchange_refund_address,
        exchange_success_address,
        exchange_success_keypair: exchange_success_keypair.clone(),
    };

    event_store.add_event(trade_id, order_taken.clone())?;
    Ok(order_taken.into())
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-htlc-funded",
    format = "application/json",
    data = "<htlc_identifier>"
)]
pub fn post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: Json<bitcoin::HtlcId>,
    event_store: State<InMemoryEventStore<TradeId>>,
    ethereum_service: State<Arc<ethereum_service::EthereumService>>,
) -> Result<(), BadRequest<String>> {
    handle_post_orders_funding(
        trade_id,
        htlc_identifier.into_inner(),
        event_store.inner(),
        ethereum_service.inner(),
    )?;
    Ok(())
}

fn handle_post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: bitcoin::HtlcId,
    event_store: &InMemoryEventStore<TradeId>,
    ethereum_service: &Arc<ethereum_service::EthereumService>,
) -> Result<(), Error> {
    let trade_funded: TradeFunded<Ethereum, Bitcoin> = TradeFunded::new(trade_id, htlc_identifier);

    event_store.add_event(trade_id.clone(), trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;

    let htlc = ethereum_htlc::Htlc::new(
        order_taken.exchange_contract_time_lock.into(),
        order_taken.exchange_refund_address,
        order_taken.client_success_address,
        order_taken.contract_secret_lock.clone(),
    );

    let offer_created_event =
        event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id.clone())?;

    let htlc_funding = offer_created_event.buy_amount.wei();

    let tx_id = ethereum_service.deploy_htlc(htlc, htlc_funding)?;
    let deployed: ContractDeployed<Ethereum, Bitcoin> =
        ContractDeployed::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, deployed)?;

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
    trade_id: TradeId,
    bitcoin_service: State<Arc<bitcoin_service::BitcoinService>>,
) -> Result<(), BadRequest<String>> {
    handle_post_revealed_secret(
        redeem_btc_notification_body.into_inner(),
        event_store.inner(),
        trade_id,
        bitcoin_service.inner(),
    )?;
    Ok(())
}

fn handle_post_revealed_secret(
    redeem_btc_notification_body: RedeemBTCNotificationBody,
    event_store: &InMemoryEventStore<TradeId>,
    trade_id: TradeId,
    bitcoin_service: &Arc<bitcoin_service::BitcoinService>,
) -> Result<(), Error> {
    let order_taken_event =
        event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;
    let offer_created_event =
        event_store.get_event::<OfferCreated<Ethereum, Bitcoin>>(trade_id.clone())?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event =
        event_store.get_event::<TradeFunded<Ethereum, Bitcoin>>(trade_id.clone())?;

    let secret: Secret = redeem_btc_notification_body.secret;

    let redeem_tx_id = bitcoin_service.redeem_htlc(
        secret,
        trade_id,
        order_taken_event,
        offer_created_event,
        trade_funded_event,
    )?;

    let contract_redeemed: ContractRedeemed<Ethereum, Bitcoin> =
        ContractRedeemed::new(trade_id, redeem_tx_id.to_string());
    event_store.add_event(trade_id, contract_redeemed)?;

    info!(
        "HTLC for {} successfully redeemed with {}",
        trade_id, redeem_tx_id
    );

    Ok(())
}
