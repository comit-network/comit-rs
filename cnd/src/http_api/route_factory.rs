use crate::{
    config::settings::AllowedOrigins, http_api, network::LocalPeerId, Facade, LocalSwapId,
};
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub fn swap_path(id: LocalSwapId) -> String {
    format!("/{}/{}", http_api::PATH, id)
}

pub fn create(facade: Facade, allowed_origins: &AllowedOrigins) -> BoxedFilter<(impl Reply,)> {
    let peer_id = facade.local_peer_id();

    let swaps = warp::path(http_api::PATH);
    let peer_id = warp::any().map(move || peer_id.clone());
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

    let get_info_siren = warp::get()
        .and(warp::path::end())
        .and(warp::header::exact("accept", "application/vnd.siren+json"))
        .and(peer_id.clone())
        .and(facade.clone())
        .and_then(http_api::routes::index::get_info_siren);

    let get_info = warp::get()
        .and(warp::path::end())
        .and(peer_id)
        .and(facade.clone())
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
