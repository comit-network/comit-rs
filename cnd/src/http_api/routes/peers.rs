use crate::{http_api::Http, network::ComitPeers, swap_protocols::Rfc003Facade};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn get_peers(dependencies: Rfc003Facade) -> Result<impl Reply, Rejection> {
    let peers = dependencies
        .comit_peers()
        .await
        .map(|(peer, addresses)| Peer {
            id: Http(peer),
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
    id: Http<PeerId>,
    endpoints: Vec<Multiaddr>,
}
