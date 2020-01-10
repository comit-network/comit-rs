mod handlers;

use self::handlers::handle_get_swaps;
use crate::{
    http_api::{problem, routes::into_rejection, Http},
    network::ListenAddresses,
    swap_protocols::Facade,
};
use futures::Future;
use futures_core::future::{FutureExt, TryFutureExt};
use http_api_problem::HttpApiProblem;
use libp2p::{Multiaddr, PeerId};
use serde::Serialize;
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Serialize, Debug)]
pub struct InfoResource {
    id: Http<PeerId>,
    listen_addresses: Vec<Multiaddr>,
}

pub fn get_info(
    id: PeerId,
    dependencies: Facade,
) -> impl Future<Item = impl Reply, Error = Rejection> {
    get_info_async(id, dependencies).boxed().compat()
}

async fn get_info_async(id: PeerId, facade: Facade) -> anyhow::Result<impl Reply, Rejection> {
    let listen_addresses: Vec<Multiaddr> = facade.listen_addresses().await.to_vec();

    Ok(warp::reply::json(&InfoResource {
        id: Http(id),
        listen_addresses,
    }))
}

pub fn get_info_siren(
    id: PeerId,
    dependencies: Facade,
) -> impl Future<Item = impl Reply, Error = Rejection> {
    get_info_siren_async(id, dependencies).boxed().compat()
}

async fn get_info_siren_async(id: PeerId, facade: Facade) -> anyhow::Result<impl Reply, Rejection> {
    let listen_addresses: Vec<Multiaddr> = facade.listen_addresses().await.to_vec();

    Ok(warp::reply::json(
        &siren::Entity::default()
            .with_properties(&InfoResource {
                id: Http(id),
                listen_addresses,
            })
            .map_err(|e| {
                log::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })
            .map_err(into_rejection)?
            .with_link(
                siren::NavigationalLink::new(&["collection"], "/swaps").with_class_member("swaps"),
            )
            .with_link(
                siren::NavigationalLink::new(&["collection", "edit"], "/swaps/rfc003")
                    .with_class_member("swaps")
                    .with_class_member("rfc003"),
            ),
    ))
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps(dependencies: Facade) -> impl Future<Item = impl Reply, Error = Rejection> {
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
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}
