use crate::network::Swarm;
use libp2p::Multiaddr;
use serde::Deserialize;
use warp::{Rejection, Reply};

#[derive(Deserialize, Debug)]
pub struct DialPeerBody {
    addresses: Vec<Multiaddr>,
}

pub async fn post_dial_addr(body: DialPeerBody, swarm: Swarm) -> Result<impl Reply, Rejection> {
    for addr in body.addresses {
        match swarm.dial_addr(addr.clone()).await {
            Ok(()) => {}
            Err(_) => tracing::warn!("connection limit hit when dialing address: {}", addr),
        }
    }
    Ok(warp::reply::reply())
}
