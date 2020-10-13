use crate::{
    network::Swarm,
    storage::{queries::get_orders_to_republish, Storage},
};
use anyhow::Result;

/// Republish all open orders to the orderbook.
pub async fn republish_open_orders(storage: Storage, swarm: Swarm) -> Result<()> {
    let open_btc_dai_orders = storage
        .db
        .do_in_transaction(get_orders_to_republish)
        .await?;

    for order in open_btc_dai_orders {
        swarm.publish_order(order).await;
    }

    Ok(())
}
