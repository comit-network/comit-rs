use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swap_protocols::{
    ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger},
    rfc003::ledger_htlc_service::{BitcoinHtlcParams, LedgerHtlcService},
};
use swaps::{
    bob_events::{
        ContractDeployed as BobContractDeployed, OrderTaken as BobOrderTaken,
        TradeFunded as BobTradeFunded,
    },
    common::TradeId,
    errors::Error,
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
    event_store: State<Arc<InMemoryEventStore<TradeId>>>,
    bitcoin_service: State<Arc<LedgerHtlcService<Bitcoin, BitcoinHtlcParams>>>,
) -> Result<(), BadRequest<String>> {
    handle_post_orders_funding(
        trade_id,
        &htlc_identifier.into_inner(),
        event_store.inner(),
        bitcoin_service.inner(),
    )?;
    Ok(())
}

fn handle_post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: &<Ethereum as Ledger>::HtlcId,
    event_store: &Arc<InMemoryEventStore<TradeId>>,
    bitcoin_service: &Arc<LedgerHtlcService<Bitcoin, BitcoinHtlcParams>>,
) -> Result<(), Error> {
    //get OrderTaken event to verify correct state
    let order_taken = event_store.get_event::<BobOrderTaken<Bitcoin, Ethereum>>(trade_id)?;

    //create new event
    let trade_funded = BobTradeFunded::<Bitcoin, Ethereum>::new(trade_id, *htlc_identifier);
    event_store.add_event(trade_id, trade_funded)?;

    let tx_id = bitcoin_service.deploy_htlc(BitcoinHtlcParams {
        refund_address: order_taken.bob_refund_address,
        success_address: order_taken.alice_success_address,
        time_lock: order_taken.bob_contract_time_lock,
        amount: order_taken.buy_amount,
        secret_hash: order_taken.contract_secret_lock,
    })?;

    let contract_deployed =
        BobContractDeployed::<Bitcoin, Ethereum>::new(trade_id, tx_id.to_string());

    event_store.add_event(trade_id, contract_deployed)?;

    Ok(())
}
