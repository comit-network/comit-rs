mod action;
mod handlers;
mod swap_state;

use crate::{
    http_api::{
        action::ActionExecutionParameters, route_factory::swap_path, routes::into_rejection,
    },
    swap_protocols::{
        rfc003::{alice::AliceSpawner, state_store::StateStore},
        MetadataStore, SwapId,
    },
};
use http_api_problem::HttpApiProblem;
use hyper::header;
use std::sync::Arc;
use warp::{Rejection, Reply};

pub use self::{
    action::{new_action_link, Action},
    handlers::*,
    swap_state::*,
};

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
pub fn action<T: MetadataStore<SwapId>, S: StateStore>(
    method: http::Method,
    id: SwapId,
    action: Action,
    query_params: ActionExecutionParameters,
    metadata_store: Arc<T>,
    state_store: Arc<S>,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    let metadata_store = metadata_store.as_ref();
    let state_store = state_store.as_ref();

    let result = match action {
        Action::Accept if method == http::Method::POST => {
            handle_accept_action(metadata_store, state_store, id, body).map(|_| None)
        }
        Action::Decline if method == http::Method::POST => {
            handle_decline_action(metadata_store, state_store, id, body).map(|_| None)
        }
        Action::Deploy if method == http::Method::GET => {
            handle_deploy_action(metadata_store, state_store, &id, &query_params).map(Some)
        }
        Action::Fund if method == http::Method::GET => {
            handle_fund_action(metadata_store, state_store, &id, &query_params).map(Some)
        }
        Action::Refund if method == http::Method::GET => {
            handle_refund_action(metadata_store, state_store, &id, &query_params).map(Some)
        }
        Action::Redeem if method == http::Method::GET => {
            handle_redeem_action(metadata_store, state_store, &id, &query_params).map(Some)
        }
        action => {
            log::debug!(target: "http-api", "Attempt to invoke {} action with http method {}, which is an invalid combination.", action, method);
            Err(HttpApiProblem::new("Invalid action invocation")
                .set_status(http::StatusCode::METHOD_NOT_ALLOWED))
        }
    };

    result
        .map(|body| warp::reply::json(&body))
        .map_err(into_rejection)
}
