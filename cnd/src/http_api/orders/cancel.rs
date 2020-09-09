use crate::{
    http_api::problem,
    network::Swarm,
    storage::{BtcDaiOrder, Storage},
};
use anyhow::Result;
use comit::OrderId;
use futures::TryFutureExt;
use warp::{Filter, Rejection, Reply};

/// The warp filter for cancelling an order.
pub fn route(
    storage: Storage,
    swarm: Swarm,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::delete()
        .and(warp::path!("orders" / OrderId))
        .and_then(move |order_id| {
            handler(order_id, storage.clone(), swarm.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(order_id: OrderId, storage: Storage, swarm: Swarm) -> Result<impl Reply> {
    let db = &storage.db;

    db.do_in_transaction(|conn| {
        use crate::storage::Order;

        let order = Order::by_order_id(conn, order_id)?;
        BtcDaiOrder::by_order(conn, &order)?.set_to_cancelled(conn)?;

        Ok(())
    })
    .await?;
    swarm.cancel_order(order_id).await;

    Ok(warp::reply())
}
