use crate::{http_api::Http, network::Network};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct PeersResource {
    peers: Vec<Peer>,
}

#[derive(Serialize, Debug)]
pub struct Peer {
    id: Http<PeerId>,
    endpoints: Vec<Multiaddr>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_peers<D: Network>(dependencies: D) -> Result<impl Reply, Rejection> {
    let peers = Network::comit_peers(&dependencies)
        .map(|(peer, addresses)| Peer {
            id: Http(peer),
            endpoints: addresses,
        })
        .collect();

    Ok(warp::reply::json(&PeersResource { peers }))
}
