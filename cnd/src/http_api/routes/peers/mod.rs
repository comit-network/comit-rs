use crate::{http_api::Http, network::ComitPeers, swap_protocols::Facade};
use futures::Future;
use futures_core::future::{FutureExt, TryFutureExt};
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
pub fn get_peers(dependencies: Facade) -> impl Future<Item = impl Reply, Error = Rejection> {
    get_peers_async(dependencies).boxed().compat()
}

async fn get_peers_async(facade: Facade) -> anyhow::Result<impl Reply, Rejection> {
    let peers = facade
        .comit_peers()
        .await
        .map(|(peer, addresses)| Peer {
            id: Http(peer),
            endpoints: addresses,
        })
        .collect();

    Ok(warp::reply::json(&PeersResource { peers }))
}
