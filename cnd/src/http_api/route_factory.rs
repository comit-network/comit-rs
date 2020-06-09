use crate::{
    config::settings::AllowedOrigins,
    http_api,
    network::LocalPeerId,
    swap_protocols::{self, rfc003::SwapId, Rfc003Facade},
    Facade, LocalSwapId,
};
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub const RFC003: &str = "rfc003";

pub fn rfc003_swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, RFC003, id)
}

pub fn swap_path(id: LocalSwapId) -> String {
    format!("/{}/{}", http_api::PATH, id)
}

pub fn new_action_link(id: &SwapId, action: &str) -> String {
    format!("{}/{}", rfc003_swap_path(*id), action)
}

pub fn create(
    rfc003_facade: Rfc003Facade,
    facade: Facade,
    allowed_origins: &AllowedOrigins,
) -> BoxedFilter<(impl Reply,)> {
    let peer_id = rfc003_facade.local_peer_id();
    let swaps = warp::path(http_api::PATH);
    let rfc003 = swaps.and(warp::path(RFC003));
    let peer_id = warp::any().map(move || peer_id.clone());
    let empty_json_body = warp::any().map(|| serde_json::json!({}));
    let rfc003_facade = warp::any().map(move || rfc003_facade.clone());
    let facade = warp::any().map(move || facade.clone());

    let cors = warp::cors()
        .allow_methods(vec!["GET", "POST"])
        .allow_header("content-type");
    let cors = match allowed_origins {
        AllowedOrigins::None => cors.allow_origins(Vec::<&str>::new()),
        AllowedOrigins::All => cors.allow_any_origin(),
        AllowedOrigins::Some(hosts) => {
            cors.allow_origins::<Vec<&str>>(hosts.iter().map(|host| host.as_str()).collect())
        }
    };

    let preflight_cors_route = warp::options().map(warp::reply);

    let rfc003_post_swap = rfc003
        .and(warp::path::end())
        .and(warp::post())
        .and(rfc003_facade.clone())
        .and(warp::body::json())
        .and_then(http_api::routes::rfc003::post_swap);

    let rfc003_get_swap = rfc003
        .and(warp::get())
        .and(rfc003_facade.clone())
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(http_api::routes::rfc003::get_swap);

    let get_swaps = swaps
        .and(warp::get())
        .and(warp::path::end())
        .and(rfc003_facade.clone())
        .and_then(http_api::routes::rfc003::get_swaps);

    let rfc003_action = warp::method()
        .and(rfc003)
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<
            swap_protocols::rfc003::actions::ActionKind,
        >())
        .and(warp::path::end())
        .and(warp::query::<http_api::action::ActionExecutionParameters>())
        .and(rfc003_facade.clone())
        .and(warp::body::json().or(empty_json_body).unify())
        .and_then(http_api::routes::rfc003::action);

    let get_peers = warp::get()
        .and(warp::path("peers"))
        .and(warp::path::end())
        .and(rfc003_facade.clone())
        .and_then(http_api::routes::peers::get_peers);

    let get_info_siren = warp::get()
        .and(warp::path::end())
        .and(warp::header::exact("accept", "application/vnd.siren+json"))
        .and(peer_id.clone())
        .and(rfc003_facade.clone())
        .and_then(http_api::routes::index::get_info_siren);

    let get_info = warp::get()
        .and(warp::path::end())
        .and(peer_id)
        .and(rfc003_facade)
        .and_then(http_api::routes::index::get_info);

    let herc20_halbit = warp::post()
        .and(warp::path!("swaps" / "herc20" / "halbit"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(facade.clone())
        .and_then(http_api::routes::index::post_herc20_halbit);

    let halbit_herc20 = warp::post()
        .and(warp::path!("swaps" / "halbit" / "herc20"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(facade.clone())
        .and_then(http_api::routes::index::post_halbit_herc20);

    let herc20_hbit = warp::post()
        .and(warp::path!("swaps" / "herc20" / "hbit"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(facade.clone())
        .and_then(http_api::routes::index::post_herc20_hbit);

    let hbit_herc20 = warp::post()
        .and(warp::path!("swaps" / "hbit" / "herc20"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(facade.clone())
        .and_then(http_api::routes::index::post_hbit_herc20);

    let get_swap = swaps
        .and(warp::get())
        .and(warp::path::param())
        .and(warp::path::end())
        .and(facade.clone())
        .and_then(http_api::routes::get_swap);

    let action_init = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("init"))
        .and(warp::path::end())
        .and(facade.clone())
        .and_then(http_api::routes::action_init);

    let action_fund = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("fund"))
        .and(warp::path::end())
        .and(facade.clone())
        .and_then(http_api::routes::action_fund);

    let action_deploy = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("deploy"))
        .and(warp::path::end())
        .and(facade.clone())
        .and_then(http_api::routes::action_deploy);

    let action_redeem = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("redeem"))
        .and(warp::path::end())
        .and(facade.clone())
        .and_then(http_api::routes::action_redeem);

    let action_refund = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("refund"))
        .and(warp::path::end())
        .and(facade)
        .and_then(http_api::routes::action_refund);

    preflight_cors_route
        .or(rfc003_get_swap)
        .or(rfc003_post_swap)
        .or(rfc003_action)
        .or(get_swaps)
        .or(get_peers)
        .or(get_info_siren)
        .or(get_info)
        .or(herc20_halbit)
        .or(halbit_herc20)
        .or(get_swap)
        .or(action_init)
        .or(action_fund)
        .or(action_deploy)
        .or(action_redeem)
        .or(action_refund)
        .or(hbit_herc20)
        .or(herc20_hbit)
        .recover(http_api::unpack_problem)
        .with(warp::log("http"))
        .with(cors)
        .boxed()
}
