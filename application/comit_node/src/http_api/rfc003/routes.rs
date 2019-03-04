pub use crate::http_api::rfc003::handlers::GetActionQueryParams;
use crate::{
    http_api::{
        problem::HttpApiProblemStdError,
        rfc003::{
            action::{new_action_link, Action},
            handlers::{
                handle_get_action, handle_get_swap, handle_get_swaps, handle_post_action,
                handle_post_swap, SwapRequestBodyKind,
            },
        },
        route_factory::swap_path,
    },
    swap_protocols::{
        rfc003::{alice::AliceSpawner, state_store::StateStore},
        MetadataStore, SwapId,
    },
};
use http_api_problem::HttpApiProblem;
use hyper::header;
use rustic_hal::HalResource;
use std::sync::Arc;
use warp::{Rejection, Reply};

fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(HttpApiProblemStdError::from(problem))
}

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap<A: AliceSpawner>(
    alice_spawner: A,
    request_body_kind: SwapRequestBodyKind,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(&alice_spawner, request_body_kind)
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response =
                warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    handle_get_swap(&metadata_store, &state_store, id)
        .map(|(swap_resource, actions)| {
            let response = HalResource::new(swap_resource);
            let response = actions.into_iter().fold(response, |acc, action| {
                let link = new_action_link(&id, action);
                acc.with_link(action, link)
            });

            Ok(warp::reply::json(&response))
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<impl Reply, Rejection> {
    handle_get_swaps(metadata_store.as_ref(), state_store.as_ref())
        .map(|swaps| {
            let response = HalResource::new("").with_resources("swaps", swaps);
            Ok(warp::reply::json(&response))
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn post_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: Action,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_post_action(
        metadata_store.as_ref(),
        state_store.as_ref(),
        id,
        action,
        body,
    )
    .map(|_| warp::reply())
    .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: Action,
    query_params: GetActionQueryParams,
) -> Result<impl Reply, Rejection> {
    handle_get_action(
        metadata_store.as_ref(),
        state_store,
        &id,
        action,
        &query_params,
    )
    .map_err(into_rejection)
}
