mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    http_api::routes::into_rejection,
    network::ListenAddresses,
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use std::sync::Arc;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    #[serde(with = "crate::http_api::serde_peer_id")]
    id: PeerId,
    listen_addresses: Vec<Multiaddr>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_info<L: ListenAddresses>(
    id: PeerId,
    get_listen_addresses: Arc<L>,
) -> Result<impl Reply, Rejection> {
    let listen_addresses: Vec<Multiaddr> = get_listen_addresses.listen_addresses().to_vec();

    Ok(warp::reply::json(&InfoResource {
        id,
        listen_addresses,
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore<SwapId>, S: StateStore>(
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
