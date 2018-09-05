use bitcoin_fee_service;
use bitcoin_rpc_client;
use bitcoin_support::{self, Network, ToP2wpkhAddress};
use common_types::{
    ledger::{
        bitcoin::{self, Bitcoin},
        ethereum::Ethereum,
        Ledger,
    },
    seconds::Seconds,
    secret::{Secret, SecretHash},
};
use ethereum_support;
use event_store::{self, EventStore, InMemoryEventStore};
use ledger_htlc_service::LedgerHtlcService;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swaps::{
    bob_events::{ContractDeployed, ContractRedeemed, OrderTaken, TradeFunded},
    common::{Error, TradeId},
};
//TODO rename Exchange to Bob
//TODO rename Client to Alice

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderRequestBody<Buy: Ledger, Sell: Ledger> {
    pub contract_secret_lock: SecretHash,
    pub client_contract_time_lock: Sell::LockDuration,
    pub client_refund_address: Sell::Address,
    pub client_success_address: Buy::Address,
    pub buy_amount: Buy::Quantity,
    pub sell_amount: Sell::Quantity,
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
        buy_amount: order_request_body.buy_amount,
        sell_amount: order_request_body.sell_amount,
    };

    event_store.add_event(trade_id, order_taken.clone())?;
    Ok(order_taken.into())
}
//TODO move this into ledger urls
#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-htlc-funded",
    format = "application/json",
    data = "<htlc_identifier>"
)]
pub fn post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: Json<bitcoin::HtlcId>,
    event_store: State<InMemoryEventStore<TradeId>>,
    ethereum_service: State<Arc<LedgerHtlcService<Ethereum>>>,
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
    ethereum_service: &Arc<LedgerHtlcService<Ethereum>>,
) -> Result<(), Error> {
    let trade_funded: TradeFunded<Ethereum, Bitcoin> = TradeFunded::new(trade_id, htlc_identifier);

    event_store.add_event(trade_id.clone(), trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;

    let tx_id = ethereum_service.deploy_htlc(
        order_taken.exchange_refund_address,
        order_taken.client_success_address,
        order_taken.exchange_contract_time_lock,
        order_taken.buy_amount,
        order_taken.contract_secret_lock.clone().into(),
    )?;

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
    bitcoin_htlc_service: State<Arc<LedgerHtlcService<Bitcoin>>>,
) -> Result<(), BadRequest<String>> {
    handle_post_revealed_secret(
        redeem_btc_notification_body.into_inner(),
        event_store.inner(),
        trade_id,
        bitcoin_htlc_service.inner(),
    )?;
    Ok(())
}

fn handle_post_revealed_secret(
    redeem_btc_notification_body: RedeemBTCNotificationBody,
    event_store: &InMemoryEventStore<TradeId>,
    trade_id: TradeId,
    bitcoin_htlc_service: &Arc<LedgerHtlcService<Bitcoin>>,
) -> Result<(), Error> {
    let order_taken_event =
        event_store.get_event::<OrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event =
        event_store.get_event::<TradeFunded<Ethereum, Bitcoin>>(trade_id.clone())?;

    let secret: Secret = redeem_btc_notification_body.secret;

    let redeem_tx_id = bitcoin_htlc_service.redeem_htlc(
        secret,
        trade_id,
        order_taken_event.exchange_success_address,
        order_taken_event.exchange_success_keypair,
        order_taken_event.client_refund_address,
        trade_funded_event.htlc_identifier,
        order_taken_event.sell_amount,
        order_taken_event.client_contract_time_lock,
    )?;

    let contract_redeemed: ContractRedeemed<Ethereum, Bitcoin> =
        ContractRedeemed::new(trade_id, redeem_tx_id.to_string());
    event_store.add_event(trade_id, contract_redeemed)?;

    Ok(())
}
