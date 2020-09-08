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
    bitcoin,
    connectors::Connectors,
    ethereum,
    http_api::{
        action::ActionResponseBody, problem, route_factory, ActionName, AlphaAbsoluteExpiry,
        AlphaLedger, AlphaProtocol, BetaAbsoluteExpiry, BetaLedger, BetaProtocol, Events, GetRole,
        Ledger, Protocol, SwapEvent,
    },
    storage::{Load, LoadAll, Storage},
    DeployAction, FundAction, InitAction, LocalSwapId, RedeemAction, RefundAction, Role,
};
use comit::Timestamp;
use serde::Serialize;
use warp::{http, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(
    id: LocalSwapId,
    storage: Storage,
    connectors: Connectors,
) -> Result<impl Reply, Rejection> {
    handle_get_swap(id, storage, connectors)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

pub async fn get_swaps(storage: Storage) -> Result<impl Reply, Rejection> {
    let swaps = async {
        let mut swaps = siren::Entity::default().with_class_member("swaps");

        for context in storage.load_all().await? {
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

async fn handle_get_swap(
    id: LocalSwapId,
    storage: Storage,
    connectors: Connectors,
) -> anyhow::Result<siren::Entity> {
    let swap_context = storage.load(id).await?;
    within_swap_context!(swap_context, {
        let swap: ActorSwap = storage.load(id).await?;
        let bitcoin_median_time_past =
            bitcoin::median_time_past(connectors.bitcoin().as_ref()).await?;
        let ethereum_latest_time = ethereum::latest_time(connectors.ethereum().as_ref()).await?;

        let swap_entity =
            make_swap_entity(id, swap, bitcoin_median_time_past, ethereum_latest_time)?;

        Ok(swap_entity)
    })
}

fn make_swap_entity<S>(
    id: LocalSwapId,
    swap: S,
    bitcoin_median_time_past: Timestamp,
    ethereum_latest_time: Timestamp,
) -> anyhow::Result<siren::Entity>
where
    S: GetRole
        + AlphaProtocol
        + BetaProtocol
        + Events
        + DeployAction
        + InitAction
        + FundAction
        + RedeemAction
        + RefundAction
        + Clone
        + AlphaLedger
        + BetaLedger
        + AlphaAbsoluteExpiry
        + BetaAbsoluteExpiry,
{
    let entity = create_swap_entity(id, &swap)?;

    match next_available_action(&swap, bitcoin_median_time_past, ethereum_latest_time)? {
        None => Ok(entity),
        Some(action) => {
            let siren_action = make_siren_action(id, action);
            Ok(entity.with_action(siren_action))
        }
    }
}

fn create_swap_entity<S>(id: LocalSwapId, swap: &S) -> anyhow::Result<siren::Entity>
where
    S: GetRole + Events + AlphaProtocol + BetaProtocol,
{
    let swap_resource = SwapResource {
        role: swap.get_role(),
        events: swap.events(), /* TODO: These events should be sorted by timestamp but we are not
                                * recording any ... */
        alpha: swap.alpha_protocol(),
        beta: swap.beta_protocol(),
    };
    let entity = siren::Entity::default()
        .with_class_member("swap")
        .with_properties(swap_resource)?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(id),
        ));

    Ok(entity)
}

fn next_available_action<S>(
    swap: &S,
    bitcoin_median_time_past: Timestamp,
    ethereum_latest_time: Timestamp,
) -> anyhow::Result<Option<ActionName>>
where
    S: GetRole
        + DeployAction
        + InitAction
        + FundAction
        + RedeemAction
        + RefundAction
        + Clone
        + AlphaLedger
        + BetaLedger
        + AlphaAbsoluteExpiry
        + BetaAbsoluteExpiry,
{
    if swap.init_action().is_ok() {
        return Ok(Some(ActionName::Init));
    }

    if swap.deploy_action().is_ok() {
        return Ok(Some(ActionName::Deploy));
    }

    if swap.fund_action().is_ok() {
        return Ok(Some(ActionName::Fund));
    }

    if swap.refund_action().is_ok() {
        let role = swap.get_role();
        let (expiry, blockchain_time) = match role {
            Role::Alice => {
                let expiry = swap.alpha_absolute_expiry().unwrap();
                let time = match swap.alpha_ledger() {
                    Ledger::Bitcoin => bitcoin_median_time_past,
                    Ledger::Ethereum => ethereum_latest_time,
                };
                (expiry, time)
            }
            Role::Bob => {
                let expiry = swap.beta_absolute_expiry().unwrap();
                let time = match swap.beta_ledger() {
                    Ledger::Bitcoin => bitcoin_median_time_past,
                    Ledger::Ethereum => ethereum_latest_time,
                };
                (expiry, time)
            }
        };

        if expiry < blockchain_time {
            tracing::debug!("We have decided it's time to refund.");
            return Ok(Some(ActionName::Refund));
        }
    }

    if swap.redeem_action().is_ok() {
        return Ok(Some(ActionName::Redeem));
    }

    Ok(None)
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
            ActionName::Init => "init",
            ActionName::Deploy => "deploy",
            ActionName::Fund => "fund",
            ActionName::Redeem => "redeem",
            ActionName::Refund => "refund",
        };
        write!(f, "{}", str)
    }
}

#[derive(Debug, Serialize)]
struct SwapResource {
    pub role: Role,
    pub events: Vec<SwapEvent>,
    pub alpha: Protocol,
    pub beta: Protocol,
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_init(id: LocalSwapId, storage: Storage) -> Result<impl Reply, Rejection> {
    handle_action_init(id, storage)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_init(
    id: LocalSwapId,
    storage: Storage,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = storage.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = storage.load(id).await?;
        let action = swap.init_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_deploy(id: LocalSwapId, storage: Storage) -> Result<impl Reply, Rejection> {
    handle_action_deploy(id, storage)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_deploy(
    id: LocalSwapId,
    storage: Storage,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = storage.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = storage.load(id).await?;
        let action = swap.deploy_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_fund(id: LocalSwapId, storage: Storage) -> Result<impl Reply, Rejection> {
    handle_action_fund(id, storage)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_fund(
    id: LocalSwapId,
    storage: Storage,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = storage.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = storage.load(id).await?;
        let action = swap.fund_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_redeem(id: LocalSwapId, storage: Storage) -> Result<impl Reply, Rejection> {
    handle_action_redeem(id, storage)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_redeem(
    id: LocalSwapId,
    storage: Storage,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = storage.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = storage.load(id).await?;
        let action = swap.redeem_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_refund(id: LocalSwapId, storage: Storage) -> Result<impl Reply, Rejection> {
    handle_action_refund(id, storage)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_refund(
    id: LocalSwapId,
    storage: Storage,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = storage.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = storage.load(id).await?;
        let action = swap.refund_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}
