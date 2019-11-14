use crate::{
    config::AllowedForeignOrigins,
    db::SaveRfc003Messages,
    http_api,
    network::{Network, SendRequest},
    seed::SwapSeed,
    swap_protocols::{
        self,
        rfc003::{state_store::StateStore, Spawn},
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
    D: Clone
        + MetadataStore
        + StateStore
        + Network
        + SendRequest
        + Spawn
        + SwapSeed
        + SaveRfc003Messages,
>(
    peer_id: PeerId,
    dependencies: D,
    allowed_foreign_origins: AllowedForeignOrigins,
) -> BoxedFilter<(impl Reply,)> {
    let swaps = warp::path(http_api::PATH);
    let rfc003 = swaps.and(warp::path(RFC003));
    let peer_id = warp::any().map(move || peer_id.clone());
    let empty_json_body = warp::any().map(|| serde_json::json!({}));
    let dependencies = warp::any().map(move || dependencies.clone());

    let cors = warp::cors()
        .allow_methods(vec!["GET", "POST"])
        .allow_header("content-type");
    let cors = match allowed_foreign_origins {
        AllowedForeignOrigins::None => cors.allow_origins(Vec::<&str>::new()),
        AllowedForeignOrigins::All => cors.allow_any_origin(),
        AllowedForeignOrigins::List(hosts) => {
            cors.allow_origins::<Vec<&str>>(hosts.iter().map(|host| host.as_str()).collect())
        }
    };

    let preflight_cors_route = warp::options().map(warp::reply);

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
