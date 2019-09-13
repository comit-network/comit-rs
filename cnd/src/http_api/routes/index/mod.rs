mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    http_api::{routes::into_rejection, Http},
    metadata_store::MetadataStore,
    network::SwarmInfo,
    state_store::StateStore,
};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use std::sync::Arc;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_info<SI: SwarmInfo>(id: PeerId, swarm_info: Arc<SI>) -> Result<impl Reply, Rejection> {
    let listen_addresses: Vec<Multiaddr> = swarm_info.listen_addresses().to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<impl Reply, Rejection> {
    handle_get_swaps(metadata_store.as_ref(), state_store.as_ref())
        .map(|swaps| {
            Ok(warp::reply::with_header(
                warp::reply::json(&swaps),
                "content-type",
                "application/vnd.siren+json",
            ))
        })
        .map_err(into_rejection)
}
