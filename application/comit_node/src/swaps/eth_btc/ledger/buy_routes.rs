use common_types::secret::Secret;
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swap_protocols::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum},
    rfc003::ledger_htlc_service::{BitcoinHtlcParams, LedgerHtlcService},
};
use swaps::{
    alice_events::ContractDeployed as AliceContractDeployed,
    bob_events::{
        ContractRedeemed as BobContractRedeemed, OrderTaken as BobOrderTaken,
        TradeFunded as BobTradeFunded,
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
    bitcoin_htlc_service: State<Arc<LedgerHtlcService<Bitcoin, BitcoinHtlcParams>>>,
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
    bitcoin_htlc_service: &Arc<LedgerHtlcService<Bitcoin, BitcoinHtlcParams>>,
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
