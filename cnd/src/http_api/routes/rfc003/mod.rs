pub mod accept;
pub mod decline;
pub mod handlers;
mod swap_state;

use crate::{
    http_api::{
        action::ActionExecutionParameters,
        route_factory::swap_path,
        routes::{
            into_rejection,
            rfc003::handlers::{handle_action, handle_get_swap, handle_post_swap},
        },
    },
    network::Network,
    swap_protocols::{rfc003::actions::ActionKind, Facade, SwapId},
};
use futures::Future;
use futures_core::future::{FutureExt, TryFutureExt};
use warp::{
    http::{self, header},
    Rejection, Reply,
};

pub use self::swap_state::{LedgerState, SwapCommunication, SwapCommunicationState, SwapState};
use crate::http_api::problem;

#[allow(clippy::needless_pass_by_value)]
pub fn post_swap<S: Network>(
    dependencies: Facade<S>,
    body: serde_json::Value,
) -> impl Future<Item = impl Reply, Error = Rejection>
where
    S: Send + Sync + 'static,
{
    handle_post_swap(dependencies, body)
        .boxed()
        .compat()
        .map(|swap_created| {
            let body = warp::reply::json(&swap_created);
            let response =
                warp::reply::with_header(body, header::LOCATION, swap_path(swap_created.id));
            warp::reply::with_status(response, warp::http::StatusCode::CREATED)
        })
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn get_swap<S>(
    dependencies: Facade<S>,
    id: SwapId,
) -> impl Future<Item = impl Reply, Error = Rejection>
where
    S: Send + Sync + 'static,
{
    handle_get_swap(dependencies, id)
        .boxed()
        .compat()
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::needless_pass_by_value)]
pub fn action<S: Network>(
    method: http::Method,
    id: SwapId,
    action_kind: ActionKind,
    query_params: ActionExecutionParameters,
    dependencies: Facade<S>,
    body: serde_json::Value,
) -> impl Future<Item = impl Reply, Error = Rejection>
where
    S: Send + Sync + 'static,
{
    handle_action(method, id, action_kind, body, query_params, dependencies)
        .boxed()
        .compat()
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}
