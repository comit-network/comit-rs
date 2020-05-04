pub mod accept;
pub mod decline;
pub mod handlers;
mod swap_state;

use crate::{
    http_api::{
        action::ActionExecutionParameters,
        route_factory,
        routes::{
            into_rejection,
            rfc003::handlers::{handle_action, handle_get_swap, handle_post_swap},
        },
    },
    swap_protocols::{
        rfc003::{actions::ActionKind, SwapId},
        Rfc003Facade,
    },
};
use warp::{
    http::{self, header},
    Rejection, Reply,
};

pub use self::swap_state::{LedgerState, SwapCommunication, SwapCommunicationState, SwapState};
use crate::http_api::problem;

#[allow(clippy::needless_pass_by_value)]
pub async fn post_swap(
    dependencies: Rfc003Facade,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_post_swap(dependencies, body)
        .await
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response = warp::reply::with_header(
                body,
                header::LOCATION,
                route_factory::rfc003_swap_path(swap_created.id),
            );
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(dependencies: Rfc003Facade, id: SwapId) -> Result<impl Reply, Rejection> {
    handle_get_swap(dependencies, id)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swaps(dependencies: Rfc003Facade) -> Result<impl Reply, Rejection> {
    handlers::handle_get_swaps(dependencies)
        .await
        .map(|swaps| {
            Ok(warp::reply::with_header(
                warp::reply::json(&swaps),
                "content-type",
                "application/vnd.siren+json",
            ))
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    query_params: ActionExecutionParameters,
    dependencies: Rfc003Facade,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    handle_action(method, id, action_kind, body, query_params, dependencies)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}
