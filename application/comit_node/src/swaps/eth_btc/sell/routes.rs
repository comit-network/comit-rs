use bitcoin_support::{self, Network, ToP2wpkhAddress};
use comit_node_api_client::OrderRequestBody;
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use ganp::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use secp256k1_support::KeyPair;
use std::sync::Arc;
use swaps::{bob_events::OrderTaken, common::TradeId, errors::Error};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrderTakenResponseBody<Buy: Ledger, Sell: Ledger> {
    pub bob_refund_address: Buy::Address,
    pub bob_success_address: Sell::Address,
    pub bob_contract_time_lock: Buy::LockDuration,
}

impl<Buy: Ledger, Sell: Ledger> From<OrderTaken<Buy, Sell>> for OrderTakenResponseBody<Buy, Sell> {
    fn from(order_taken_event: OrderTaken<Buy, Sell>) -> Self {
        OrderTakenResponseBody {
            bob_refund_address: order_taken_event.bob_refund_address.into(),
            bob_success_address: order_taken_event.bob_success_address.into(),
            bob_contract_time_lock: order_taken_event.bob_contract_time_lock,
        }
    }
}

#[post(
    "/trades/ETH-BTC/<trade_id>/sell-orders",
    format = "application/json",
    data = "<order_request_body>"
)]
pub fn post_sell_orders(
    trade_id: TradeId,
    order_request_body: Json<OrderRequestBody<Bitcoin, Ethereum>>,
    event_store: State<Arc<InMemoryEventStore<TradeId>>>,
    bob_success_keypair: State<KeyPair>,
    bob_refund_address: State<ethereum_support::Address>,
    network: State<Network>,
) -> Result<Json<OrderTakenResponseBody<Bitcoin, Ethereum>>, BadRequest<String>> {
    let order_taken = handle_post_sell_orders(
        trade_id,
        order_request_body.into_inner(),
        event_store.inner(),
        bob_success_keypair.inner(),
        bob_refund_address.inner(),
        network.inner(),
    )?;

    Ok(Json(order_taken.into()))
}

fn handle_post_sell_orders(
    trade_id: TradeId,
    order_request_body: OrderRequestBody<Bitcoin, Ethereum>,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    bob_success_keypair: &KeyPair,
    bob_refund_address: &ethereum_support::Address,
    network: &Network,
) -> Result<OrderTaken<Bitcoin, Ethereum>, Error> {
    let alice_refund_address: ethereum_support::Address =
        order_request_body.alice_refund_address.into();

    let bob_success_address = bob_success_keypair
        .public_key()
        .clone()
        .to_p2wpkh_address(*network)
        .into();

    let order_taken = OrderTaken {
        uid: trade_id,
        contract_secret_lock: order_request_body.contract_secret_lock,
        alice_contract_time_lock: order_request_body.alice_contract_time_lock,
        bob_contract_time_lock: bitcoin_support::Blocks::from(60 * 60 * 12),
        alice_refund_address,
        alice_success_address: order_request_body.alice_success_address,
        bob_refund_address: bob_success_address, //TODO rename refund and success variables but for now they are simply swapped
        bob_success_address: *bob_refund_address, //TODO rename refund and success variables but for now they are simply swapped
        bob_success_keypair: bob_success_keypair.clone(),
        buy_amount: order_request_body.buy_amount,
        sell_amount: order_request_body.sell_amount,
    };

    event_store.add_event(trade_id, order_taken.clone())?;

    Ok(order_taken)
}
