use crate::{
    http_api,
    swap_protocols::{self, rfc003::state_store, MetadataStore, SwapId},
};
use libp2p::Transport;
use serde_json::json;
use std::sync::Arc;
use tokio::{io::AsyncRead, prelude::AsyncWrite};
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub const RFC003: &str = "rfc003";
pub fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, RFC003, id)
}

pub fn create<
    T: MetadataStore<SwapId>,
    S: state_store::StateStore,
    TTransport: Transport + Send + 'static,
    TSubstream: AsyncWrite + AsyncRead + Send + 'static,
>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    protocol_dependencies: swap_protocols::alice::ProtocolDependencies<
        T,
        S,
        TTransport,
        TSubstream,
    >,
    origin_auth: String,
) -> BoxedFilter<(impl Reply,)>
where
    <TTransport as Transport>::Listener: Send,
    <TTransport as Transport>::Error: Send,
{
    let path = warp::path(http_api::PATH);
    let rfc003 = path.and(warp::path(RFC003));
    let metadata_store = warp::any().map(move || metadata_store.clone());
    let state_store = warp::any().map(move || state_store.clone());
    let empty_json_body = warp::any().map(|| json!({}));
    let protocol_dependencies = warp::any().map(move || protocol_dependencies.clone());

    let rfc003_post_swap = rfc003
        .and(warp::path::end())
        .and(warp::post2())
        .and(protocol_dependencies.clone())
        .and(warp::body::json())
        .and_then(http_api::routes::rfc003::post_swap);

    let rfc003_get_swap = rfc003
        .and(warp::get2())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(http_api::routes::rfc003::get_swap);

    let get_swaps = path
        .and(warp::get2())
        .and(warp::path::end())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and_then(http_api::routes::index::get_swaps);

    let rfc003_post_action = rfc003
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<
            http_api::routes::rfc003::action::ActionName,
        >())
        .and(warp::post2())
        .and(warp::path::end())
        .and(warp::body::json().or(empty_json_body).unify())
        .and_then(http_api::routes::rfc003::post_action);

    let rfc003_get_action = rfc003
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<
            http_api::routes::rfc003::action::ActionName,
        >())
        .and(warp::query::<http_api::routes::rfc003::GetActionQueryParams>())
        .and(warp::get2())
        .and(warp::path::end())
        .and_then(http_api::routes::rfc003::get_action);

    //    let get_peers = warp::path("peers")
    //        .and(comit_connection_pool.clone())
    //        .and(warp::get2())
    //        .and(warp::path::end())
    //        .and_then(http_api::routes::peers::get_peers);

    let preflight_cors_route = warp::options().map(warp::reply);

    let cors = warp::cors()
        .allow_origin(origin_auth.as_str())
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["content-type"]);

    preflight_cors_route
        .or(rfc003_get_swap)
        .or(rfc003_post_swap)
        .or(rfc003_post_action)
        .or(rfc003_get_action)
        .or(get_swaps)
        //        .or(get_peers)
        .with(warp::log("http"))
        .with(cors)
        .recover(http_api::unpack_problem)
        .boxed()
}
