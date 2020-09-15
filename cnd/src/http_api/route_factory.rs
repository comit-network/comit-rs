use crate::{
    config::{AllowedOrigins, Settings},
    connectors::Connectors,
    http_api,
    http_api::{
        dial_addr, halbit_herc20, hbit_herc20, herc20_halbit, herc20_hbit, info, markets, orders,
        peers, swaps, tokens,
    },
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
    connectors: Connectors,
    settings: &Settings,
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
    let connectors = warp::any().map(move || connectors.clone());

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

    let preflight_cors_route = warp::options().map(warp::reply);

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

    let herc20_halbit = warp::post()
        .and(warp::path!("swaps" / "herc20" / "halbit"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(storage_filter.clone())
        .and(swarm_filter.clone())
        .and_then(herc20_halbit::post_swap);

    let halbit_herc20 = warp::post()
        .and(warp::path!("swaps" / "halbit" / "herc20"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(storage_filter.clone())
        .and(swarm_filter.clone())
        .and_then(halbit_herc20::post_swap);

    let herc20_hbit = warp::post()
        .and(warp::path!("swaps" / "herc20" / "hbit"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(storage_filter.clone())
        .and(swarm_filter.clone())
        .and_then(herc20_hbit::post_swap);

    let hbit_herc20 = warp::post()
        .and(warp::path!("swaps" / "hbit" / "herc20"))
        .and(warp::path::end())
        .and(warp::body::json())
        .and(storage_filter.clone())
        .and(swarm_filter.clone())
        .and_then(hbit_herc20::post_swap);

    let get_swap = swaps
        .and(warp::get())
        .and(warp::path::param())
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and(connectors)
        .and_then(swaps::get_swap);

    let get_swaps = warp::get()
        .and(swaps)
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::get_swaps);

    let action_init = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("init"))
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::action_init);

    let action_fund = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("fund"))
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::action_fund);

    let action_deploy = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("deploy"))
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::action_deploy);

    let action_redeem = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("redeem"))
        .and(warp::path::end())
        .and(storage_filter.clone())
        .and_then(swaps::action_redeem);

    let action_refund = swaps
        .and(warp::get())
        .and(warp::path::param::<LocalSwapId>())
        .and(warp::path("refund"))
        .and(warp::path::end())
        .and(storage_filter)
        .and_then(swaps::action_refund);

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
        .or(herc20_halbit)
        .or(halbit_herc20)
        .or(get_swap)
        .or(get_swaps)
        .or(action_init)
        .or(action_fund)
        .or(action_deploy)
        .or(action_redeem)
        .or(action_refund)
        .or(hbit_herc20)
        .or(herc20_hbit)
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
        .with(warp::log("http"))
        .with(cors)
        .boxed()
}
