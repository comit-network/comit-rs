use common_types::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    secret::Secret,
};
use ethereum_service;
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swaps::{
    common::{Error, TradeId},
    events::{ContractRedeemed, OrderTaken, TradeFunded},
};

#[derive(Deserialize, Debug)]
pub struct SellOrderHtlcDeployedNotification {
    contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/sell-order-htlc-funded",
    format = "application/json",
    data = "<htlc_identifier>"
)]
pub fn post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: Json<<Ethereum as Ledger>::HtlcId>,
    event_store: State<InMemoryEventStore<TradeId>>,
) -> Result<(), BadRequest<String>> {
    handle_post_orders_funding(trade_id, htlc_identifier.into_inner(), event_store.inner())?;
    Ok(())
}

fn handle_post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: <Ethereum as Ledger>::HtlcId,
    event_store: &InMemoryEventStore<TradeId>,
) -> Result<(), Error> {
    //get OrderTaken event to verify correct state
    let _order_taken = event_store.get_event::<OrderTaken<Bitcoin, Ethereum>>(trade_id.clone())?;

    //create new event
    let trade_funded: TradeFunded<Ethereum> = TradeFunded {
        uid: trade_id,
        htlc_identifier,
    };
    event_store.add_event(trade_id.clone(), trade_funded)?;
    //TODO Finish this and implement bitcoin service for deploying the bitcoin htlc

    Ok(())
}

#[derive(Deserialize)]
pub struct RedeemETHNotificationBody {
    pub secret: Secret,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/sell-order-secret-revealed",
    format = "application/json",
    data = "<redeem_eth_notification_body>"
)]
pub fn post_revealed_secret(
    redeem_eth_notification_body: Json<RedeemETHNotificationBody>,
    event_store: State<InMemoryEventStore<TradeId>>,
    trade_id: TradeId,
    ethereum_service: State<Arc<ethereum_service::EthereumService>>,
) -> Result<(), BadRequest<String>> {
    handle_post_revealed_secret(
        redeem_eth_notification_body.into_inner(),
        event_store.inner(),
        trade_id,
        ethereum_service.inner(),
    )?;

    Ok(())
}

fn handle_post_revealed_secret(
    redeem_eth_notification_body: RedeemETHNotificationBody,
    event_store: &InMemoryEventStore<TradeId>,
    trade_id: TradeId,
    ethereum_service: &Arc<ethereum_service::EthereumService>,
) -> Result<(), Error> {
    let trade_funded = event_store.get_event::<TradeFunded<Ethereum>>(trade_id.clone())?;

    let tx_id = ethereum_service.redeem_htlc(
        redeem_eth_notification_body.secret,
        trade_funded.htlc_identifier,
    )?;
    let deployed: ContractRedeemed<Ethereum> = ContractRedeemed::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, deployed)?;
    Ok(())
}
