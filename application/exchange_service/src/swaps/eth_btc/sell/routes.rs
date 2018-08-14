use ethereum_htlc;
use ethereum_service;
use ethereum_support;
use event_store::InMemoryEventStore;
use reqwest;
use rocket::{response::status::BadRequest, State};
use rocket_contrib::Json;
use std::sync::Arc;
use swaps::{eth_btc::common::Error, TradeId};

#[derive(Deserialize)]
pub struct SellOrderHtlcDeployedNotification {
    contract_address: ethereum_support::Address,
}

#[post(
    "/trades/ETH-BTCLN/<trade_id>/sell-order-htlc-deployed",
    format = "application/json",
    data = "<sell_order_htlc_deployed_notification>"
)]
pub fn post_sell_order_htlc_deployed(
    trade_id: TradeId,
    sell_order_htlc_deployed_notification: Json<SellOrderHtlcFundedNotification>,
    event_store: State<InMemoryEventStore<TradeId>>,
    ethereum_service: State<Arc<ethereum_service::EthereumService>>,
) -> Result<(), BadRequest<String>> {
    handle_post_buy_order_funding(
        trade_id,
        buy_order_htlc_funded_notification.into_inner(),
        event_store.inner(),
        ethereum_service.inner(),
    )?;
    Ok(())
}

fn handle_post_sell_order_htlc_deployed(
    trade_id: TradeId,
    sell_order_htlc_deployed_notification: SellOrderHtlcDeployedNotification,
    event_store: &InMemoryEventStore<TradeId>,
    ethereum_service: &Arc<ethereum_service::EthereumService>,
) -> Result<(), Error> {
    let trade_funded = TradeFunded {
        uid: trade_id,
        transaction_id: buy_order_htlc_funded_notification.transaction_id.clone(),
        vout: buy_order_htlc_funded_notification.vout,
    };

    event_store.add_event(trade_id.clone(), trade_funded)?;

    let order_taken = event_store.get_event::<OrderTaken>(trade_id.clone())?;

    let htlc = ethereum_htlc::Htlc::new(
        order_taken.exchange_contract_time_lock,
        order_taken.exchange_refund_address,
        order_taken.client_success_address,
        order_taken.contract_secret_lock.clone(),
    );

    let offer_created_event = event_store.get_event::<OfferCreated>(trade_id.clone())?;

    let htlc_funding = offer_created_event.eth_amount.wei();

    let tx_id = ethereum_service.deploy_htlc(htlc, htlc_funding)?;

    event_store.add_event(
        trade_id,
        ContractDeployed {
            uid: trade_id,
            transaction_id: tx_id,
        },
    )?;

    Ok(())
}
