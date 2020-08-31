use crate::{http_api::problem, Facade};
use anyhow::Result;
use comit::OrderId;
use futures::TryFutureExt;
use warp::{Filter, Rejection, Reply};

/// The warp filter for cancelling an order.
pub fn route(facade: Facade) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::delete()
        .and(warp::path!("orders" / OrderId))
        .and_then(move |order_id| {
            handler(order_id, facade.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(order_id: OrderId, facade: Facade) -> Result<impl Reply> {
    let db = &facade.storage.db;
    let swarm = &facade.swarm;

    db.do_in_transaction(|conn| {
        use crate::storage::Order;

        Order::by_order_id(conn, order_id)?.cancel(conn)?;

        Ok(())
    })
    .await?;
    swarm.cancel_order(order_id).await;

    Ok(warp::reply())
}
