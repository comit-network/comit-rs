use crate::{
    http_api::{make_order_entity, problem},
    Facade,
};
use anyhow::Result;
use comit::OrderId;
use futures::TryFutureExt;
use warp::{Filter, Rejection, Reply};

/// The warp filter for getting a single order.
pub fn route(facade: Facade) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path!("orders" / OrderId))
        .and_then(move |order_id| {
            handler(order_id, facade.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(order_id: OrderId, facade: Facade) -> Result<impl Reply> {
    let db = &facade.storage.db;
    let properties = db
        .do_in_transaction(|conn| {
            use crate::storage::{BtcDaiOrder, Order};

            let order = Order::by_order_id(conn, order_id)?;
            let btc_dai_order = BtcDaiOrder::by_order(conn, &order)?;

            Ok((order, btc_dai_order))
        })
        .await?
        .into();

    Ok(warp::reply::json(&make_order_entity(properties)?))
}
