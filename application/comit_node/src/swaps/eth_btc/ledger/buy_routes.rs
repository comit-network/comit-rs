use common_types::secret::Secret;
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use ganp::ledger::{
    bitcoin::{self, Bitcoin},
    ethereum::Ethereum,
};
use ledger_htlc_service::LedgerHtlcService;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swaps::{
    alice_events::ContractDeployed as AliceContractDeployed,
    bob_events::{
        ContractDeployed as BobContractDeployed, ContractRedeemed as BobContractRedeemed,
        OrderTaken as BobOrderTaken, TradeFunded as BobTradeFunded,
    },
    common::TradeId,
    errors::Error,
};

#[derive(Deserialize)]
pub struct AliceContractDeployedRequestBody {
    pub contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTC/<trade_id>/buy-order-contract-deployed",
    format = "application/json",
    data = "<contract_deployed_request_body>"
)]
pub fn post_contract_deployed(
    trade_id: TradeId,
    contract_deployed_request_body: Json<AliceContractDeployedRequestBody>,
    event_store: State<Arc<InMemoryEventStore<TradeId>>>,
) -> Result<(), BadRequest<String>> {
    let event_store = event_store.inner();
    handle_post_contract_deployed(
        event_store,
        trade_id,
        contract_deployed_request_body.into_inner().contract_address,
    )?;

    Ok(())
}

fn handle_post_contract_deployed(
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    uid: TradeId,
    address: ethereum_support::Address,
) -> Result<(), Error> {
    let deployed: AliceContractDeployed<Ethereum, Bitcoin> =
        AliceContractDeployed::new(uid, address);
    event_store.add_event(uid, deployed)?;

    Ok(())
}

//TODO move this into ledger urls
#[post(
    "/trades/ETH-BTC/<_trade_id>/buy-order-htlc-funded",
    format = "application/json",
    data = "<htlc_identifier>"
)]
pub fn post_orders_funding(
    _trade_id: TradeId,
    htlc_identifier: Json<bitcoin::HtlcId>,
    event_store: State<Arc<InMemoryEventStore<TradeId>>>,
    ethereum_service: State<Arc<LedgerHtlcService<Ethereum>>>,
) -> Result<(), BadRequest<String>> {
    let event_store = event_store.inner();
    handle_post_orders_funding(
        // TODO HACK: Ignore trade id in post and just the first one
        // from event_store because the poker is not giving us the
        // right trade id anymore!
        event_store.keys().next().unwrap(),
        htlc_identifier.into_inner(),
        event_store,
        ethereum_service.inner(),
    )?;
    Ok(())
}

fn handle_post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: bitcoin::HtlcId,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    ethereum_service: &Arc<LedgerHtlcService<Ethereum>>,
) -> Result<(), Error> {
    let trade_funded: BobTradeFunded<Ethereum, Bitcoin> =
        BobTradeFunded::new(trade_id, htlc_identifier);

    event_store.add_event(trade_id.clone(), trade_funded)?;

    let order_taken = event_store.get_event::<BobOrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;

    let tx_id = ethereum_service.deploy_htlc(
        order_taken.bob_refund_address,
        order_taken.alice_success_address,
        order_taken.bob_contract_time_lock,
        order_taken.buy_amount,
        order_taken.contract_secret_lock.clone().into(),
    )?;

    let deployed: BobContractDeployed<Ethereum, Bitcoin> =
        BobContractDeployed::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, deployed)?;

    Ok(())
}

#[derive(Deserialize)]
pub struct RedeemBTCNotificationBody {
    pub secret: Secret,
}

#[post(
    "/trades/ETH-BTC/<_trade_id>/buy-order-secret-revealed",
    format = "application/json",
    data = "<redeem_btc_notification_body>"
)]
pub fn post_revealed_secret(
    redeem_btc_notification_body: Json<RedeemBTCNotificationBody>,
    event_store: State<Arc<InMemoryEventStore<TradeId>>>,
    _trade_id: TradeId,
    bitcoin_htlc_service: State<Arc<LedgerHtlcService<Bitcoin>>>,
) -> Result<(), BadRequest<String>> {
    let event_store = event_store.inner();
    handle_post_revealed_secret(
        redeem_btc_notification_body.into_inner(),
        event_store,
        // TODO HACK: Ignore trade id in post and just the first one
        // from event_store because the poker is not giving us the
        // right trade id anymore!
        event_store.keys().next().unwrap(),
        bitcoin_htlc_service.inner(),
    )?;
    Ok(())
}

fn handle_post_revealed_secret(
    redeem_btc_notification_body: RedeemBTCNotificationBody,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    trade_id: TradeId,
    bitcoin_htlc_service: &Arc<LedgerHtlcService<Bitcoin>>,
) -> Result<(), Error> {
    let order_taken_event =
        event_store.get_event::<BobOrderTaken<Ethereum, Bitcoin>>(trade_id.clone())?;
    // TODO: Maybe if this fails we keep the secret around anyway and steal money early?
    let trade_funded_event =
        event_store.get_event::<BobTradeFunded<Ethereum, Bitcoin>>(trade_id.clone())?;

    let secret: Secret = redeem_btc_notification_body.secret;

    let redeem_tx_id = bitcoin_htlc_service.redeem_htlc(
        secret,
        trade_id,
        order_taken_event.bob_success_address,
        order_taken_event.bob_success_keypair,
        order_taken_event.alice_refund_address,
        trade_funded_event.htlc_identifier,
        order_taken_event.sell_amount,
        order_taken_event.alice_contract_time_lock,
    )?;

    let contract_redeemed: BobContractRedeemed<Ethereum, Bitcoin> =
        BobContractRedeemed::new(trade_id, redeem_tx_id.to_string());
    event_store.add_event(trade_id, contract_redeemed)?;

    Ok(())
}
