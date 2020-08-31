use crate::{
    http_api::{
        orders::{make_order_entity, OrderProperties},
        problem,
    },
    storage::all_open_btc_dai_orders,
    Facade,
};
use anyhow::Result;
use futures::TryFutureExt;
use warp::{Filter, Rejection, Reply};

/// The warp filter for listing all open orders.
pub fn route(facade: Facade) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get().and(warp::path!("orders")).and_then(move || {
        handler(facade.clone())
            .map_err(problem::from_anyhow)
            .map_err(warp::reject::custom)
    })
}

async fn handler(facade: Facade) -> Result<impl Reply> {
    let db = &facade.storage.db;
    let orders = db
        .do_in_transaction(|conn| all_open_btc_dai_orders(conn))
        .await?;

    let mut open_orders = siren::Entity::default();

    for entity in orders
        .into_iter()
        .map(OrderProperties::from)
        .map(make_order_entity)
    {
        open_orders.push_sub_entity(siren::SubEntity::from_entity(entity?, &["item"]));
    }

    Ok(warp::reply::json(&open_orders))
}
