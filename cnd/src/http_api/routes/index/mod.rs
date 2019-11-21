mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    db::{DetermineTypes, Retrieve},
    http_api::{routes::into_rejection, Http},
    network::Network,
    swap_protocols::rfc003::state_store::StateStore,
};
use futures::Future;
use futures_core::future::{FutureExt, TryFutureExt};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_info<D: Network>(id: PeerId, dependencies: D) -> Result<impl Reply, Rejection> {
    let listen_addresses: Vec<Multiaddr> = Network::listen_addresses(&dependencies).to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<D: DetermineTypes + Retrieve + StateStore>(
    dependencies: D,
) -> impl Future<Item = impl Reply, Error = Rejection> {
    handle_get_swaps(dependencies)
        .boxed()
        .compat()
        .map(|swaps| {
            Ok(warp::reply::with_header(
                warp::reply::json(&swaps),
                "content-type",
                "application/vnd.siren+json",
            ))
        })
        .map_err(into_rejection)
}
