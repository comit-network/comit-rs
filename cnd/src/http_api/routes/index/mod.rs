mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    connector::Connect,
    http_api::{routes::into_rejection, Http},
    network::Network,
};
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_info<C: Connect>(id: PeerId, con: C) -> Result<impl Reply, Rejection> {
    let listen_addresses: Vec<Multiaddr> = Network::listen_addresses(&con).to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<C: Connect>(con: C) -> Result<impl Reply, Rejection> {
    handle_get_swaps(con)
        .map(|swaps| {
            Ok(warp::reply::with_header(
                warp::reply::json(&swaps),
                "content-type",
                "application/vnd.siren+json",
            ))
        })
        .map_err(into_rejection)
}
