use crate::{
    http_api,
    network::Network,
    swap_protocols::{
        self,
        rfc003::{
            alice::{InitiateRequest, SendRequest},
            bob::SpawnBob,
            state_store::StateStore,
        },
        MetadataStore, SwapId,
    },
};
use libp2p::PeerId;
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub const RFC003: &str = "rfc003";

pub fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, RFC003, id)
}

pub fn new_action_link(id: &SwapId, action: &str) -> String {
    format!("{}/{}", swap_path(*id), action)
}

pub fn create<
    D: MetadataStore + StateStore + SpawnBob + Network + Clone + InitiateRequest + SendRequest,
>(
    origin_auth: String,
    peer_id: PeerId,
    dependencies: D,
) -> BoxedFilter<(impl Reply,)> {
    let swaps = warp::path(http_api::PATH);
    let rfc003 = swaps.and(warp::path(RFC003));
    let peer_id = warp::any().map(move || peer_id.clone());
    let empty_json_body = warp::any().map(|| serde_json::json!({}));
    let dependencies = warp::any().map(move || dependencies.clone());

    let rfc003_post_swap = rfc003
        .and(warp::path::end())
        .and(warp::post2())
        .and(dependencies.clone())
        .and(warp::body::json())
        .and_then(http_api::routes::rfc003::post_swap);

    let rfc003_get_swap = rfc003
        .and(warp::get2())
        .and(dependencies.clone())
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(http_api::routes::rfc003::get_swap);

    let get_swaps = swaps
        .and(warp::get2())
        .and(warp::path::end())
        .and(dependencies.clone())
        .and_then(http_api::routes::index::get_swaps);

    let rfc003_action = warp::method()
        .and(rfc003)
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<
            swap_protocols::rfc003::actions::ActionKind,
        >())
        .and(warp::path::end())
        .and(warp::query::<http_api::action::ActionExecutionParameters>())
        .and(dependencies.clone())
        .and(warp::body::json().or(empty_json_body).unify())
        .and_then(http_api::routes::rfc003::action);

    let get_peers = warp::get2()
        .and(warp::path("peers"))
        .and(warp::path::end())
        .and(dependencies.clone())
        .and_then(http_api::routes::peers::get_peers);

    let get_info = warp::get2()
        .and(warp::path::end())
        .and(peer_id.clone())
        .and(dependencies.clone())
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
