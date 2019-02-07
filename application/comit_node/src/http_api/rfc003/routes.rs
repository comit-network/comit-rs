pub use crate::http_api::rfc003::handlers::{GetAction, GetActionQueryParams, PostAction};
use crate::{
    http_api::{
        self,
        problem::HttpApiProblemStdError,
        rfc003::handlers::{
            handle_get_action, handle_get_swap, handle_get_swaps, handle_post_action,
            handle_post_swap, SwapRequestBodyKind,
        },
    },
    swap_protocols::{
        rfc003::{alice::AliceSpawner, state_store::StateStore, SecretSource},
        MetadataStore, SwapId,
    },
};
use http_api_problem::HttpApiProblem;
use hyper::header;
use rustic_hal::HalResource;
use std::sync::Arc;
use warp::{Rejection, Reply};

pub const PROTOCOL_NAME: &str = "rfc003";
pub fn swap_path(id: SwapId) -> String {
    format!("/{}/{}/{}", http_api::PATH, PROTOCOL_NAME, id)
}

fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(HttpApiProblemStdError::from(problem))
}

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap<A: AliceSpawner>(
    alice_spawner: A,
    secret_source: Arc<dyn SecretSource>,
    request_body_kind: SwapRequestBodyKind,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(&alice_spawner, secret_source.as_ref(), request_body_kind)
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response =
                warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    handle_get_swap(&metadata_store, &state_store, &id)
        .map(|(swap_resource, actions)| {
            let mut response = HalResource::new(swap_resource);
            for action in actions {
                let route = format!("{}/{}", swap_path(id), action);
                response.with_link(action, route);
            }
            Ok(warp::reply::json(&response))
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swaps<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
) -> Result<impl Reply, Rejection> {
    handle_get_swaps(metadata_store.as_ref(), state_store.as_ref())
        .map(|swaps| {
            let mut response = HalResource::new("");
            response.with_resources("swaps", swaps);
            Ok(warp::reply::json(&response))
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn post_action<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    secret_source: Arc<dyn SecretSource>,
    id: SwapId,
    action: PostAction,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_post_action(
        metadata_store.as_ref(),
        state_store.as_ref(),
        secret_source.as_ref(),
        id,
        action,
        body,
    )
    .map(|_| warp::reply())
    .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_action<T: MetadataStore<SwapId>, S: StateStore<SwapId>>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    action: GetAction,
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
