use crate::{http_api::serde_peer_id, network::Swarm};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn get_peers(swarm: Swarm) -> Result<impl Reply, Rejection> {
    let peers = swarm
        .connected_peers()
        .await
        .map(|(peer, addresses)| Peer {
            id: peer,
            endpoints: addresses,
        })
        .collect();

    Ok(warp::reply::json(&PeersResource { peers }))
}

#[derive(Serialize, Debug)]
pub struct PeersResource {
    peers: Vec<Peer>,
}

#[derive(Serialize, Debug)]
pub struct Peer {
    #[serde(with = "serde_peer_id")]
    id: PeerId,
    endpoints: Vec<Multiaddr>,
}
