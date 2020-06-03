pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset,
    http_api::{
        action::ActionResponseBody,
        halight, hbit, herc20, problem,
        protocol::{
            AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams, BetaAbsoluteExpiry,
            BetaEvents, BetaLedger, BetaParams, GetRole, Ledger, LedgerEvents,
        },
        route_factory, ActionNotFound, AliceSwap, BobSwap, Http, Swap,
    },
    storage::Load,
    DeployAction, Facade, FundAction, InitAction, LocalSwapId, Protocol, RedeemAction,
    RefundAction, Role,
};
use anyhow::bail;
use http_api_problem::HttpApiProblem;
use serde::Serialize;
use warp::{http, http::StatusCode, Rejection, Reply};

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(problem)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_swap(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_get_swap(facade, swap_id)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

pub async fn handle_get_swap(
    facade: Facade,
    swap_id: LocalSwapId,
) -> anyhow::Result<siren::Entity> {
    match facade.load(swap_id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                halight::Finalized,
            > = facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> =
                facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                halight::Finalized,
                herc20::Finalized,
            > = facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> =
                facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            > = facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsFunder,
            > = facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsFunder,
                herc20::Finalized,
            > = facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsRedeemer,
                herc20::Finalized,
            > = facade.load(swap_id).await?;
            make_swap_entity(facade.clone(), swap_id, swap).await
        }
        _ => unimplemented!("other combinations not suported yet"),
    }
}

async fn make_swap_entity<S>(
    facade: Facade,
    swap_id: LocalSwapId,
    swap: S,
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

    let mut entity = create_swap_entity(swap_id, role)?;
    add_params(&mut entity, &swap)?;

    match (swap.alpha_events(), swap.beta_events()) {
        (Some(alpha), Some(beta)) => {
            add_events(&mut entity, alpha, beta)?;

            match next_available_action(facade, &swap).await? {
                None => Ok(entity),
                Some(action) => {
                    let siren_action = make_siren_action(swap_id, action);
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
        .with_properties(swap_resource)
        .map_err(|e| {
            tracing::error!("failed to set properties of entity: {:?}", e);
            HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?
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
            .with_properties(alpha_params)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["alpha"],
    );
    entity.push_sub_entity(alpha_params_sub);

    let beta_params = swap.beta_params();
    let beta_params_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("parameters")
            .with_properties(beta_params)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
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
            .with_properties(alpha_events)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["alpha"],
    );
    entity.push_sub_entity(alpha_state_sub);

    let beta_state_sub = siren::SubEntity::from_entity(
        siren::Entity::default()
            .with_class_member("state")
            .with_properties(beta_events)
            .map_err(|e| {
                tracing::error!("failed to set properties of entity: {:?}", e);
                HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
            })?,
        &["beta"],
    );
    entity.push_sub_entity(beta_state_sub);

    Ok(())
}

async fn next_available_action<S>(facade: Facade, swap: &S) -> anyhow::Result<Option<ActionName>>
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

fn make_siren_action(swap_id: LocalSwapId, action_name: ActionName) -> siren::Action {
    siren::Action {
        name: action_name.to_string(),
        class: vec![],
        method: Some(http::Method::GET),
        href: format!("/swaps/{}/{}", swap_id, action_name),
        title: None,
        _type: None,
        fields: vec![],
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActionName {
    Init,
    Deploy,
    Fund,
    Redeem,
    Refund,
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
pub async fn action_init(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_init(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_init(id: LocalSwapId, facade: Facade) -> anyhow::Result<ActionResponseBody> {
    let action = match facade.load(id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                halight::Finalized,
            > = facade.load(id).await?;
            swap.init_action()?
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> =
                facade.load(id).await?;
            swap.init_action()?
        }
        _ => bail!(ActionNotFound),
    };

    let response = ActionResponseBody::from(action);

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_deploy(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_deploy(swap_id, facade)
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
    let action = match facade.load(id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                halight::Finalized,
            > = facade.load(id).await?;
            swap.deploy_action()?
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> =
                facade.load(id).await?;
            swap.deploy_action()?
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            > = facade.load(id).await?;
            swap.deploy_action()?
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsRedeemer,
                herc20::Finalized,
            > = facade.load(id).await?;
            swap.deploy_action()?
        }
        _ => bail!(ActionNotFound),
    };

    let response = ActionResponseBody::from(action);

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_fund(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_fund(swap_id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_fund(id: LocalSwapId, facade: Facade) -> anyhow::Result<ActionResponseBody> {
    let response = match facade.load(id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                halight::Finalized,
            > = facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> =
                facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                halight::Finalized,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> =
                facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            > = facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsFunder,
            > = facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsFunder,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsRedeemer,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.fund_action()?;
            ActionResponseBody::from(action)
        }
        _ => anyhow::bail!(ActionNotFound),
    };

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_redeem(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_redeem(swap_id, facade)
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
    let response = match facade.load(id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                halight::Finalized,
            > = facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Erc20, asset::Bitcoin, herc20::Finalized, halight::Finalized> =
                facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                halight::Finalized,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> =
                facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            > = facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsFunder,
            > = facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsFunder,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsRedeemer,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.redeem_action()?;
            ActionResponseBody::from(action)
        }
        _ => return Err(ActionNotFound.into()),
    };

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_refund(swap_id: LocalSwapId, facade: Facade) -> Result<impl Reply, Rejection> {
    handle_action_refund(swap_id, facade)
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
    let response = match facade.load(id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                halight::Finalized,
            > = facade.load(id).await?;

            let action = swap.refund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, halight::Finalized, herc20::Finalized> =
                facade.load(id).await?;

            let action = swap.refund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsRedeemer,
            > = facade.load(id).await?;

            let action = swap.refund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Hbit,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Erc20,
                asset::Bitcoin,
                herc20::Finalized,
                hbit::FinalizedAsFunder,
            > = facade.load(id).await?;

            let action = swap.refund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsFunder,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.refund_action()?;
            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Hbit,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<
                asset::Bitcoin,
                asset::Erc20,
                hbit::FinalizedAsRedeemer,
                herc20::Finalized,
            > = facade.load(id).await?;

            let action = swap.refund_action()?;
            ActionResponseBody::from(action)
        }
        _ => anyhow::bail!(ActionNotFound),
    };

    Ok(response)
}
