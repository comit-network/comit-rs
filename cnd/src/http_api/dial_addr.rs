use crate::{facade::Facade, http_api::problem};
use libp2p::Multiaddr;
use serde::Deserialize;
use warp::{Rejection, Reply};

#[derive(Deserialize, Debug)]
pub struct DialPeerBody {
    addresses: Vec<Multiaddr>,
}

pub async fn post_dial_addr(
    body: serde_json::Value,
    mut facade: Facade,
) -> Result<impl Reply, Rejection> {
    let body = DialPeerBody::deserialize(&body)
        .map_err(anyhow::Error::new)
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)?;

    for addr in body.addresses {
        facade.dial_addr(addr).await;
    }

    // Best effort, assume we got a connection.
    let sub = facade.subscribe().await;
    if !sub {
        tracing::warn!("failed to subscribe to orderbook gossipsub for BTC/DAI");
    }

    Ok(warp::reply::reply())
}
