use common_types::ledger::{bitcoin::Bitcoin, ethereum::Ethereum, Ledger};
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::str::FromStr;
use swaps::{
    common::{Error, TradeId},
    events::{OrderTaken, TradeFunded},
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

    Ok(())
}
