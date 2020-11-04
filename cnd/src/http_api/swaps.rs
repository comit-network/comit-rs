//! The REST API exposes the "/swaps" endpoint for three purposes:
//!
//! 1. To create a swap: POST requests can create a swap on the swap
//!    protocol endpoint e.g., /swaps/hbit/herc20
//!
//! 2. To fetch swap details: GET requests can fetch swap details on
//!    the "/swaps/:swap_id" endpoint
//!
//! 3. To fetch swap actions: GET requests can fetch an appropriate swap
//!    action on the action endpoint e.g., "/swaps/:swap_id/fund"

use crate::{
    bitcoin_fees::BitcoinFees,
    http_api::{
        action::ActionResponseBody, problem, route_factory, ActionName, ActionNotFound, Protocol,
        SwapEvent,
    },
    storage::{queries::get_active_swap_contexts, Load, Storage},
    LocalSwapId, Role,
};
use comit::swap::Action;
use serde::Serialize;
use warp::{http, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(id: LocalSwapId, storage: Storage) -> Result<impl Reply, Rejection> {
    handle_get_swap(id, storage)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

pub async fn get_swaps(storage: Storage) -> Result<impl Reply, Rejection> {
    let swaps = async {
        let mut swaps = siren::Entity::default().with_class_member("swaps");

        for context in storage
            .db
            .do_in_transaction(get_active_swap_contexts)
            .await?
        {
            swaps.push_sub_entity(siren::SubEntity::from_link(siren::EntityLink {
                class: vec![],
                title: None,
                rel: vec![String::from("item")],
                href: format!("/swaps/{}", context.id),
                _type: None,
            }));
        }

        Ok(swaps)
    }
    .await
    .map_err(problem::from_anyhow)
    .map_err(warp::reject::custom)?;

    Ok(warp::reply::json(&swaps))
}

async fn handle_get_swap(id: LocalSwapId, storage: Storage) -> anyhow::Result<siren::Entity> {
    let resource = storage.load(id).await?;
    let next_action = storage.next_action.lock().await.get(&id).cloned();

    let swap_entity = make_swap_entity(id, resource, next_action).await?;

    Ok(swap_entity)
}

async fn make_swap_entity(
    id: LocalSwapId,
    swap_resource: SwapResource,
    next_action: Option<Action>,
) -> anyhow::Result<siren::Entity> {
    let entity = siren::Entity::default()
        .with_class_member("swap")
        .with_properties(swap_resource)?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(id),
        ));

    match next_action {
        None => Ok(entity),
        Some(action) => {
            let siren_action = make_siren_action(id, action.into());
            Ok(entity.with_action(siren_action))
        }
    }
}

fn make_siren_action(id: LocalSwapId, action_name: ActionName) -> siren::Action {
    siren::Action {
        name: action_name.to_string(),
        class: vec![],
        method: Some(http::Method::GET),
        href: format!("/swaps/{}/{}", id, action_name),
        title: None,
        _type: None,
        fields: vec![],
    }
}

impl std::fmt::Display for ActionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let str = match self {
            ActionName::Deploy => "deploy",
            ActionName::Fund => "fund",
            ActionName::Redeem => "redeem",
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug, Serialize)]
pub struct SwapResource {
    pub role: Role,
    pub events: Vec<SwapEvent>,
    pub alpha: Protocol,
    pub beta: Protocol,
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action<DUMMY>(
    id: LocalSwapId,
    _: DUMMY,
    storage: Storage,
    bitcoin_fees: BitcoinFees,
) -> Result<impl Reply, Rejection> {
    handle_action(id, storage, bitcoin_fees)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action(
    id: LocalSwapId,
    storage: Storage,
    bitcoin_fees: BitcoinFees,
) -> anyhow::Result<ActionResponseBody> {
    let action = storage
        .next_action
        .lock()
        .await
        .get(&id)
        .cloned()
        .ok_or(ActionNotFound)?;
    let btc_per_vbyte = bitcoin_fees.get_per_vbyte_rate().await?;

    ActionResponseBody::from_action(action, btc_per_vbyte)
}
