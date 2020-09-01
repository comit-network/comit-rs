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
    http_api::{
        action::ActionResponseBody,
        problem,
        protocol::{
            ActionName, AlphaAbsoluteExpiry, AlphaLedger, AlphaProtocol, BetaAbsoluteExpiry,
            BetaLedger, BetaProtocol, Events, GetRole, Ledger, Protocol, SwapEvent,
        },
        route_factory, Http,
    },
    storage::{Load, LoadAll},
    DeployAction, Facade, FundAction, InitAction, LocalSwapId, RedeemAction, RefundAction, Role,
};
use serde::Serialize;
use warp::{http, Rejection, Reply};

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_get_swap(id, facade)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

pub async fn get_swaps(facade: Facade) -> Result<impl Reply, Rejection> {
    let swaps = async {
        let mut swaps = siren::Entity::default().with_class_member("swaps");

        for context in facade.storage.load_all().await? {
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

async fn handle_get_swap(id: LocalSwapId, facade: Facade) -> anyhow::Result<siren::Entity> {
    let swap_context = facade.load(id).await?;
    within_swap_context!(swap_context, {
        let swap: ActorSwap = facade.load(id).await?;
        let swap_entity = make_swap_entity(id, swap, facade.clone()).await?;

        Ok(swap_entity)
    })
}

async fn make_swap_entity<S>(
    id: LocalSwapId,
    swap: S,
    facade: Facade,
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

    match next_available_action(&swap, facade).await? {
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
        role: Http(swap.get_role()),
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

async fn next_available_action<S>(swap: &S, facade: Facade) -> anyhow::Result<Option<ActionName>>
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
                    Ledger::Bitcoin => facade.bitcoin_median_time_past().await?,
                    Ledger::Ethereum => facade.ethereum_latest_time().await?,
                };
                (expiry, time)
            }
            Role::Bob => {
                let expiry = swap.beta_absolute_expiry().unwrap();
                let time = match swap.beta_ledger() {
                    Ledger::Bitcoin => facade.bitcoin_median_time_past().await?,
                    Ledger::Ethereum => facade.ethereum_latest_time().await?,
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
    pub role: Http<Role>,
    pub events: Vec<SwapEvent>,
    pub alpha: Protocol,
    pub beta: Protocol,
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_init(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_init(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_init(id: LocalSwapId, facade: Facade) -> anyhow::Result<ActionResponseBody> {
    let swap_context = facade.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = facade.load(id).await?;
        let action = swap.init_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_deploy(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_deploy(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_deploy(
    id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = facade.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = facade.load(id).await?;
        let action = swap.deploy_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_fund(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_fund(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_fund(id: LocalSwapId, facade: Facade) -> anyhow::Result<ActionResponseBody> {
    let swap_context = facade.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = facade.load(id).await?;
        let action = swap.fund_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_redeem(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_redeem(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_redeem(
    id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = facade.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = facade.load(id).await?;
        let action = swap.redeem_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_refund(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_refund(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(warp::reject::custom)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_refund(
    id: LocalSwapId,
    facade: Facade,
) -> anyhow::Result<ActionResponseBody> {
    let swap_context = facade.load(id).await?;
    let response = within_swap_context!(swap_context, {
        let swap: ActorSwap = facade.load(id).await?;
        let action = swap.refund_action()?;
        ActionResponseBody::from(action)
    });

    Ok(response)
}
