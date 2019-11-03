pub mod accept;
pub mod decline;
mod handlers;
mod swap_state;

use crate::{
    comit_client::Client,
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
    swap_protocols::{
        rfc003::{
            actions::ActionKind,
            alice::{AliceSpawn, InitiateRequest},
            state_store::StateStore,
        },
        MetadataStore, SwapId,
    },
};
use hyper::header;
use warp::{Rejection, Reply};

pub use self::swap_state::{LedgerState, SwapCommunication, SwapCommunicationState, SwapState};
use crate::{network::Network, swap_protocols::rfc003::bob::BobSpawn};

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap<D: AliceSpawn + InitiateRequest + Client>(
    dependencies: D,
    request_body_kind: SwapRequestBodyKind,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(&dependencies, request_body_kind)
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response =
                warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<D: MetadataStore + StateStore>(
    dependencies: D,
    id: SwapId,
) -> Result<impl Reply, Rejection> {
    handle_get_swap(&dependencies, id)
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn action<D: MetadataStore + StateStore + BobSpawn + Network>(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    query_params: ActionExecutionParameters,
    dependencies: D,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_action(method, id, action_kind, body, query_params, dependencies)
        .map(|body| warp::reply::json(&body))
        .map_err(into_rejection)
}
