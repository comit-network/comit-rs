use crate::{
    comit_client::ClientPool,
    http_api::{self, rfc003::action::GetActionQueryParams},
    seed::Seed,
    swap_protocols::{
        rfc003::{alice::AliceSpawner, state_store, SecretSource},
        MetadataStore, SwapId,
    },
};
use std::sync::Arc;
use warp::{self, filters::BoxedFilter, Filter, Reply};

pub fn create<
    T: MetadataStore<SwapId>,
    S: state_store::StateStore<SwapId>,
    A: AliceSpawner,
    C: ClientPool,
>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    alice_spawner: Arc<A>,
    seed: Seed,
    comit_client_pool: Arc<C>,
) -> BoxedFilter<(impl Reply,)> {
    let seed = Arc::new(seed);
    let path = warp::path(http_api::PATH);
    let rfc003 = path.and(warp::path(http_api::rfc003::swap::PROTOCOL_NAME));
    let metadata_store = warp::any().map(move || metadata_store.clone());
    let rfc003_secret_gen = warp::any().map(move || seed.clone() as Arc<dyn SecretSource>);
    let state_store = warp::any().map(move || state_store.clone());
    let empty_json_body = warp::any().map(|| json!({}));
    let alice_spawner = warp::any().map(move || alice_spawner.clone());
    let comit_client_pool = warp::any().map(move || comit_client_pool.clone());

    let rfc003_post_swap = rfc003
        .and(warp::path::end())
        .and(warp::post2())
        .and(alice_spawner)
        .and(rfc003_secret_gen.clone())
        .and(warp::body::json())
        .and_then(http_api::rfc003::swap::post_swap);

    let rfc003_get_swap = rfc003
        .and(warp::get2())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(http_api::rfc003::swap::get_swap);

    let get_swaps = path
        .and(warp::get2())
        .and(warp::path::end())
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and_then(http_api::rfc003::swap::get_swaps);

    let rfc003_post_action = rfc003
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(rfc003_secret_gen.clone())
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<http_api::rfc003::action::PostAction>())
        .and(warp::post2())
        .and(warp::path::end())
        .and(warp::body::json().or(empty_json_body).unify())
        .and_then(http_api::rfc003::action::post);

    let rfc003_get_action = rfc003
        .and(metadata_store.clone())
        .and(state_store.clone())
        .and(warp::path::param::<SwapId>())
        .and(warp::path::param::<http_api::rfc003::action::GetAction>())
        .and(warp::query::<GetActionQueryParams>())
        .and(warp::get2())
        .and(warp::path::end())
        .and_then(http_api::rfc003::action::get);

    let get_peers = warp::path("peers")
        .and(comit_client_pool.clone())
        .and(warp::get2())
        .and(warp::path::end())
        .and_then(http_api::peers);

    rfc003_get_swap
        .or(rfc003_post_swap)
        .or(rfc003_post_action)
        .or(rfc003_get_action)
        .or(get_swaps)
        .or(get_peers)
        .with(warp::log("http"))
        .recover(http_api::unpack_problem)
        .boxed()
}
