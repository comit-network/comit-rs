pub mod accept;
pub mod decline;
mod handlers;
mod swap_state;

use crate::{
    connector::Connect,
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
    swap_protocols::{rfc003::actions::ActionKind, SwapId},
};
use hyper::header;
use warp::{Rejection, Reply};

pub use self::swap_state::{LedgerState, SwapCommunication, SwapCommunicationState, SwapState};

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap<C: Connect>(
    con: C,
    request_body_kind: SwapRequestBodyKind,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(con, request_body_kind)
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response =
                warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<C: Connect>(con: C, id: SwapId) -> Result<impl Reply, Rejection> {
    handle_get_swap(con, id)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn action<C: Connect>(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    query_params: ActionExecutionParameters,
    con: C,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_action(method, id, action_kind, body, query_params, con)
        .map(|body| warp::reply::json(&body))
        .map_err(into_rejection)
}
