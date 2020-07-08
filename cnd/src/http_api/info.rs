use crate::{
    http_api::{problem, Http},
    network::{ListenAddresses, LocalPeerId},
    Facade,
};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

/// Basic HTTP GET request on the root endpoint.
pub async fn get_info(facade: Facade) -> Result<impl Reply, Rejection> {
    let peer_id = facade.local_peer_id();
    let listen_addresses = facade.listen_addresses().await.to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(peer_id),
        listen_addresses,
    }))
}

/// HTTP GET request, for a siren document, on the root endpoint.
pub async fn get_info_siren(facade: Facade) -> Result<impl Reply, Rejection> {
    let peer_id = facade.local_peer_id();
    let listen_addresses = facade.listen_addresses().await.to_vec();

    Ok(warp::reply::json(
        &siren::Entity::default()
            .with_properties(&InfoResource {
                id: Http(peer_id),
                listen_addresses,
            })
            .map_err(anyhow::Error::from)
            .map_err(problem::from_anyhow)
            .map_err(warp::reject::custom)?
            .with_link(
                siren::NavigationalLink::new(&["collection"], "/swaps").with_class_member("swaps"),
            ),
    ))
}

#[derive(Serialize, Debug)]
struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}
