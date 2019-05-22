pub mod action;
mod handlers;
mod swap_state;

use crate::{
    http_api::{route_factory::swap_path, routes::into_rejection},
    swap_protocols::{
        rfc003::{alice::AliceSpawner, state_store::StateStore},
        MetadataStore, SwapId,
    },
};
use hyper::header;
use std::sync::Arc;
use warp::{Rejection, Reply};

pub use self::{handlers::*, swap_state::*};

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
    handle_get_swap(metadata_store.as_ref(), state_store.as_ref(), id)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn accept_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_accept_action(metadata_store.as_ref(), state_store.as_ref(), id, body)
        .map(|_| warp::reply())
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn decline_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_decline_action(metadata_store.as_ref(), state_store.as_ref(), id, body)
        .map(|_| warp::reply())
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn deploy_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    query_params: ActionExecutionParameters,
) -> Result<impl Reply, Rejection> {
    handle_deploy_action(metadata_store.as_ref(), state_store, &id, &query_params)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn fund_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    query_params: ActionExecutionParameters,
) -> Result<impl Reply, Rejection> {
    handle_fund_action(metadata_store.as_ref(), state_store, &id, &query_params)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn refund_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    query_params: ActionExecutionParameters,
) -> Result<impl Reply, Rejection> {
    handle_refund_action(metadata_store.as_ref(), state_store, &id, &query_params)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn redeem_action<T: MetadataStore<SwapId>, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
    query_params: ActionExecutionParameters,
) -> Result<impl Reply, Rejection> {
    handle_redeem_action(metadata_store.as_ref(), state_store, &id, &query_params)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}
