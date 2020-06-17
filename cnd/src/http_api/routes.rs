pub mod index;
pub mod peers;

use crate::{
    http_api::{
        action::ActionResponseBody,
        problem,
        protocol::{
            ActionName, AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams,
            BetaAbsoluteExpiry, BetaEvents, BetaLedger, BetaParams, GetRole, Ledger, LedgerEvents,
        },
        route_factory, Http,
    },
    storage::Load,
    DeployAction, Facade, FundAction, InitAction, LocalSwapId, RedeemAction, RefundAction, Role,
};
use http_api_problem::HttpApiProblem;
use serde::Serialize;
use warp::{http, Rejection, Reply};

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(problem)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_get_swap(id, facade)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

pub async fn handle_get_swap(id: LocalSwapId, facade: Facade) -> anyhow::Result<siren::Entity> {
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
        + AlphaParams
        + BetaParams
        + AlphaEvents
        + BetaEvents
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
    let role = swap.get_role();

    let mut entity = create_swap_entity(id, role)?;
    add_params(&mut entity, &swap)?;

    match (swap.alpha_events(), swap.beta_events()) {
        (Some(alpha), Some(beta)) => {
            add_events(&mut entity, alpha, beta)?;

            match next_available_action(&swap, facade).await? {
                None => Ok(entity),
                Some(action) => {
                    let siren_action = make_siren_action(id, action);
                    Ok(entity.with_action(siren_action))
                }
            }
        }
        _ => Ok(entity),
    }
}

fn create_swap_entity(id: LocalSwapId, role: Role) -> anyhow::Result<siren::Entity> {
    let swap_resource = SwapResource { role: Http(role) };
    let entity = siren::Entity::default()
        .with_class_member("swap")
        .with_properties(swap_resource)?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(id),
        ));

    Ok(entity)
}

fn add_params<S>(entity: &mut siren::Entity, swap: &S) -> anyhow::Result<()>
where
    S: AlphaParams + BetaParams,
{
    let alpha_params = swap.alpha_params();
    let alpha_params_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("parameters")
            .with_properties(alpha_params)?,
        &["alpha"],
    );
    entity.push_sub_entity(alpha_params_sub);

    let beta_params = swap.beta_params();
    let beta_params_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("parameters")
            .with_properties(beta_params)?,
        &["beta"],
    );
    entity.push_sub_entity(beta_params_sub);

    Ok(())
}

fn add_events(
    entity: &mut siren::Entity,
    alpha_events: LedgerEvents,
    beta_events: LedgerEvents,
) -> anyhow::Result<()> {
    let alpha_state_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("state")
            .with_properties(alpha_events)?,
        &["alpha"],
    );
    entity.push_sub_entity(alpha_state_sub);

    let beta_state_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("state")
            .with_properties(beta_events)?,
        &["beta"],
    );
    entity.push_sub_entity(beta_state_sub);

    Ok(())
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum SwapStatus {
    Created,
    InProgress,
    Swapped,
    NotSwapped,
}

#[derive(Debug, Serialize)]
struct SwapResource {
    pub role: Http<Role>,
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_init(id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_init(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
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
        .map_err(into_rejection)
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
        .map_err(into_rejection)
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
        .map_err(into_rejection)
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
        .map_err(into_rejection)
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
