use crate::{
    bitcoin_fees::BitcoinFees,
    config::{AllowedOrigins, Settings},
    http_api,
    http_api::{dial_addr, info, markets, orders, peers, swaps, tokens},
    network::Swarm,
    storage::Storage,
    LocalSwapId,
};
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub fn swap_path(id: LocalSwapId) -> String {
    format!("/{}/{}", http_api::PATH, id)
}

pub fn create(
    swarm: Swarm,
    storage: Storage,
    settings: &Settings,
    bitcoin_fees: BitcoinFees,
    network: comit::Network,
) -> BoxedFilter<(impl Reply,)> {
    let swaps = warp::path(http_api::PATH);
    let swarm_filter = warp::any().map({
        let swarm = swarm.clone();
        move || swarm.clone()
    });
    let storage_filter = warp::any().map({
        let storage = storage.clone();
        move || storage.clone()
    });
    let bitcoin_fees = warp::any().map(move || bitcoin_fees.clone());
    let preflight_cors_route = warp::options().map(warp::reply);

    let cors = warp::cors()
        .allow_methods(vec!["GET", "POST"])
        .allow_header("content-type");
    let cors = match &settings.http_api.cors.allowed_origins {
        AllowedOrigins::None => cors.allow_origins(Vec::<&str>::new()),
        AllowedOrigins::All => cors.allow_any_origin(),
        AllowedOrigins::Some(hosts) => {
            cors.allow_origins::<Vec<&str>>(hosts.iter().map(|host| host.as_str()).collect())
        }
    };

    let get_info = warp::get()
        .and(warp::path::end())
        .and(swarm_filter.clone())
        .and_then(info::get_info);

    let get_info_siren = warp::get()
        .and(warp::path::end())
        .and(warp::header::exact("accept", "application/vnd.siren+json"))
        .and(swarm_filter.clone())
        .and_then(info::get_info_siren);

    let get_peers = warp::get()
        .and(warp::path("peers"))
        .and(warp::path::end())
        .and(swarm_filter.clone())
        .and_then(peers::get_peers);

    let get_swap = swaps
        .and(warp::get())
        .and(warp::path::param())
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::get_swap);

    let get_swaps = warp::get()
        .and(swaps)
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::get_swaps);

    let action = warp::get()
        .and(swaps)
        .and(warp::path::param())
        .and(
            warp::path("fund")
                .or(warp::path("deploy"))
                .or(warp::path("redeem"))
                .or(warp::path("refund")),
        )
        .and(warp::path::end())
        .and(storage_filter)
        .and(bitcoin_fees)
        .and_then(swaps::action);

    let post_dial_addr = warp::post()
        .and(warp::path!("dial"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(swarm_filter)
        .and_then(dial_addr::post_dial_addr);

    preflight_cors_route
        .or(get_peers)
        .or(get_info_siren)
        .or(get_info)
        .or(get_swap)
        .or(get_swaps)
        .or(action)
        .or(orders::make_btc_dai(
            storage.clone(),
            swarm.clone(),
            settings.clone(),
            network,
        ))
        .or(orders::get_single(storage.clone()))
        .or(orders::list_open(storage.clone()))
        .or(orders::cancel(storage, swarm.clone()))
        .or(tokens::list(settings.clone()))
        .or(markets::get_btc_dai(swarm, network))
        .or(post_dial_addr)
        .recover(http_api::unpack_problem)
        .with(warp::trace(|info| {
            tracing::error_span!(
                "request",
                method = %info.method(),
                path = %info.path(),
            )
        }))
        .with(cors)
        .boxed()
}
