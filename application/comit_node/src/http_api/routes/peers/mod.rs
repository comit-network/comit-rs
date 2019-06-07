use crate::network::SwarmInfo;
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use std::sync::Arc;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct PeersResource {
    peers: Vec<Peer>,
}

#[derive(Serialize, Debug)]
pub struct Peer {
    #[serde(with = "crate::http_api::serde_peer_id")]
    id: PeerId,
    endpoints: Vec<Multiaddr>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_peers<BP: SwarmInfo>(get_bam_peers: Arc<BP>) -> Result<impl Reply, Rejection> {
    let peers = get_bam_peers
        .bam_peers()
        .map(|(peer, addresses)| Peer {
            id: peer,
            endpoints: addresses,
        })
        .collect();

    Ok(warp::reply::json(&PeersResource { peers }))
}
