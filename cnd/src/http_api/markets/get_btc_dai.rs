use crate::{
    facade::Facade,
    http_api::{amount::Amount, problem, serde_peer_id},
};
use anyhow::{Context, Result};
use comit::{order::Denomination, OrderId, Position};
use futures::TryFutureExt;
use libp2p::PeerId;
use serde::Serialize;
use warp::{reply, Filter, Rejection, Reply};

/// The warp filter for getting the BTC/DAI market view.
pub fn route(facade: Facade) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::get()
        .and(warp::path!("markets" / "BTC-DAI"))
        .and_then(move || {
            handler(facade.clone())
                .map_err(problem::from_anyhow)
                .map_err(warp::reject::custom)
        })
}

async fn handler(facade: Facade) -> Result<impl Reply> {
    let mut orders = siren::Entity::default();
    let local_peer_id = facade.swarm.local_peer_id();

    for (maker, order) in facade.swarm.btc_dai_market().await {
        let market_item = siren::Entity::default()
            .with_properties(MarketItem {
                id: order.id,
                quantity: Amount::btc(order.quantity),
                price: Amount::dai(order.price(Denomination::WeiPerBtc)),
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
