use crate::{http_api, metadata_store::MetadataStore, network::SwarmInfo, state_store::StateStore};
use comit::{Client, SwapId};
use libp2p::PeerId;
use std::sync::Arc;
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub const RFC003: &str = "rfc003";

pub fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, RFC003, id)
}

pub fn new_action_link(id: &SwapId, action: &str) -> String {
    format!("{}/{}", swap_path(*id), action)
}

pub fn create<T: MetadataStore, S: StateStore, C: Client, SI: SwarmInfo>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    protocol_dependencies: crate::swap_protocols::alice::ProtocolDependencies<T, S, C>,
    origin_auth: String,
    swarm_info: Arc<SI>,
    peer_id: PeerId,
) -> BoxedFilter<(impl Reply,)> {
    let swaps = warp::path(http_api::PATH);
    let rfc003 = swaps.and(warp::path(RFC003));
    let metadata_store = warp::any().map(move || Arc::clone(&metadata_store));
    let state_store = warp::any().map(move || Arc::clone(&state_store));
    let protocol_dependencies = warp::any().map(move || protocol_dependencies.clone());
    let swarm_info = warp::any().map(move || Arc::clone(&swarm_info));
    let peer_id = warp::any().map(move || peer_id.clone());
    let empty_json_body = warp::any().map(|| serde_json::json!({}));

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

    let get_swaps = swaps
        .and(warp::get2())
        .and(warp::path::end())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and_then(http_api::routes::index::get_swaps);

    let rfc003_action = warp::method()
        .and(rfc003)
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<comit::rfc003::actions::ActionKind>())
        .and(warp::path::end())
        .and(warp::query::<http_api::action::ActionExecutionParameters>())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::body::json().or(empty_json_body).unify())
        .and_then(http_api::routes::rfc003::action);

    let get_peers = warp::get2()
        .and(warp::path("peers"))
        .and(warp::path::end())
        .and(swarm_info.clone())
        .and_then(http_api::routes::peers::get_peers);

    let get_info = warp::get2()
        .and(warp::path::end())
        .and(peer_id.clone())
        .and(swarm_info.clone())
        .and_then(http_api::routes::index::get_info);

    let preflight_cors_route = warp::options().map(warp::reply);

    let cors = warp::cors()
        .allow_origin(origin_auth.as_str())
        .allow_methods(vec!["GET", "POST"])
        .allow_headers(vec!["content-type"]);

    preflight_cors_route
        .or(rfc003_get_swap)
        .or(rfc003_post_swap)
        .or(rfc003_action)
        .or(get_swaps)
        .or(get_peers)
        .or(get_info)
        .recover(http_api::unpack_problem)
        .with(warp::log("http"))
        .with(cors)
        .boxed()
}
