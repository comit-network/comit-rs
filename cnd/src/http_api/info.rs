use crate::{
    http_api::{problem, serde_peer_id},
    network::Swarm,
};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

/// Basic HTTP GET request on the root endpoint.
pub async fn get_info(swarm: Swarm) -> Result<impl Reply, Rejection> {
    let peer_id = swarm.local_peer_id();
    let listen_addresses = swarm.listen_addresses().await.to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: peer_id,
        listen_addresses,
    }))
}

/// HTTP GET request, for a siren document, on the root endpoint.
pub async fn get_info_siren(swarm: Swarm) -> Result<impl Reply, Rejection> {
    let peer_id = swarm.local_peer_id();
    let listen_addresses = swarm.listen_addresses().await.to_vec();

    Ok(warp::reply::json(
        &siren::Entity::default()
            .with_properties(&InfoResource {
                id: peer_id,
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
    #[serde(with = "serde_peer_id")]
    id: PeerId,
    listen_addresses: Vec<Multiaddr>,
}
