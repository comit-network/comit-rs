mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    http_api::routes::into_rejection,
    swap_protocols::{rfc003::state_store::StateStore, MetadataStore, SwapId},
};
use libp2p::PeerId;
use rustic_hal::HalResource;
use serde::Serialize;
use std::sync::Arc;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    #[serde(with = "crate::http_api::serde_peer_id")]
    id: PeerId,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_info(id: PeerId) -> Result<impl Reply, Rejection> {
    Ok(warp::reply::json(&InfoResource { id }))
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<impl Reply, Rejection> {
    handle_get_swaps(metadata_store.as_ref(), state_store.as_ref())
        .map(|swaps| {
            let response = HalResource::new("").with_resources("swaps", swaps);
            Ok(warp::reply::json(&response))
        })
        .map_err(into_rejection)
}
