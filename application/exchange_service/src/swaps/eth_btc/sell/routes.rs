use common_types::ledger::{ethereum::Ethereum, Ledger};
use ethereum_support;
use event_store::{EventStore, InMemoryEventStore};
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use swaps::{
    common::{Error, TradeId},
    events::TradeFunded,
};

#[derive(Deserialize)]
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
    //TODO: pass a lnd service similar to ethereum_service
) -> Result<(), BadRequest<String>> {
    handle_post_orders_funding(
        trade_id,
        htlc_identifier.into_inner(),
        event_store.inner(),
        //TODO: pass a lnd service similar to ethereum_service
    )?;
    Ok(())
}

fn handle_post_orders_funding(
    trade_id: TradeId,
    htlc_identifier: <Ethereum as Ledger>::HtlcId,
    event_store: &InMemoryEventStore<TradeId>,
    //TODO: pass a lnd service similar to ethereum_service
) -> Result<(), Error> {
    let trade_funded: TradeFunded<Ethereum> = TradeFunded {
        uid: trade_id,
        htlc_identifier,
    };

    event_store.add_event(trade_id.clone(), trade_funded)?;
    //TODO: Probably need to pay over LN now :)

    Ok(())
}
