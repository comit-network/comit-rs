pub mod accept;
pub mod decline;
mod handlers;
mod swap_state;

pub use self::swap_state::{LedgerState, SwapCommunication, SwapCommunicationState, SwapState};
use crate::{
    http_api::{
        action::ActionExecutionParameters,
        route_factory::swap_path,
        routes::{
            into_rejection,
            rfc003::handlers::{
                handle_action, handle_get_swap, handle_post_swap, SwapRequestBodyKind,
            },
        },
    },
    metadata_store::MetadataStore,
    state_store::StateStore,
    swap_protocols::rfc003::alice::AliceSpawner,
};
use comit::{rfc003::actions::ActionKind, SwapId};
use hyper::header;
use std::sync::Arc;
use warp::{Rejection, Reply};

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
pub fn get_swap<T: MetadataStore, S: StateStore>(
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    handle_get_swap(metadata_store.as_ref(), state_store.as_ref(), id)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn action<T: MetadataStore, S: StateStore>(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    query_params: ActionExecutionParameters,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    let metadata_store = metadata_store.as_ref();
    let state_store = state_store.as_ref();

    handle_action(
        method,
        id,
        action_kind,
        body,
        query_params,
        metadata_store,
        state_store,
    )
    .map(|body| warp::reply::json(&body))
    .map_err(into_rejection)
}
