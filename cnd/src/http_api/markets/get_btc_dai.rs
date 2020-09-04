use crate::{
    http_api::{problem, serde_peer_id, Amount},
    network::Swarm,
};
use anyhow::{Context, Result};
use comit::{OrderId, Position};
use futures::TryFutureExt;
use libp2p::PeerId;
use serde::Serialize;
use warp::{reply, Filter, Rejection, Reply};

/// The warp filter for getting the BTC/DAI market view.
pub fn route(swarm: Swarm) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path!("markets" / "BTC-DAI"))
        .and_then(move || {
            handler(swarm.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(swarm: Swarm) -> Result<impl Reply> {
    let mut orders = siren::Entity::default();
    let local_peer_id = swarm.local_peer_id();

    for (maker, order) in swarm.btc_dai_market_safe_expiries().await {
        let market_item = siren::Entity::default()
            .with_properties(MarketItem {
                id: order.id,
                quantity: Amount::from(order.quantity),
                price: Amount::from(order.price),
                ours: maker == local_peer_id,
                maker,
                position: order.position,
            })
            .context("failed to serialize market item sub entity")?;

        orders.push_sub_entity(siren::SubEntity::from_entity(market_item, &["item"]))
    }

    Ok(reply::json(&orders))
}

#[derive(Clone, Debug, Serialize)]
struct MarketItem {
    id: OrderId,
    #[serde(with = "serde_peer_id")]
    maker: PeerId,
    ours: bool,
    position: Position,
    quantity: Amount,
    price: Amount,
}
