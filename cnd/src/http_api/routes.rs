pub mod index;
pub mod peers;
pub mod post;
pub mod rfc003;

use crate::{
    http_api::{
        action::ActionResponseBody,
        halight::HalightFinalized,
        herc20::Herc20Finalized,
        problem,
        protocol::{GetAlphaEvents, GetAlphaParams, GetBetaEvents, GetBetaParams, GetRole},
        route_factory, ActionNotFound, AliceSwap, BobSwap, Http, Swap,
    },
    storage::Load,
    swap_protocols::{
        DeployAction, Facade, FundAction, InitAction, LocalSwapId, RedeemAction, RefundAction, Role,
    },
};
use ::comit::Protocol;
use comit::asset;
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
            let swap: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(swap_id).await?;
            make_swap_entity(swap_id, swap)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(swap_id).await?;
            make_swap_entity(swap_id, swap)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<asset::Bitcoin, asset::Erc20, HalightFinalized, Herc20Finalized> =
                facade.load(swap_id).await?;
            make_swap_entity(swap_id, swap)
        }
        Swap {
            alpha: Protocol::Halight,
            beta: Protocol::Herc20,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Bitcoin, asset::Erc20, HalightFinalized, Herc20Finalized> =
                facade.load(swap_id).await?;
            make_swap_entity(swap_id, swap)
        }
        _ => unimplemented!("other combinations not suported yet"),
    }
}

fn make_swap_entity<S>(swap_id: LocalSwapId, swap: S) -> anyhow::Result<siren::Entity>
where
    S: GetRole
        + GetAlphaParams
        + GetBetaParams
        + GetAlphaEvents
        + GetBetaEvents
        + DeployAction
        + InitAction
        + FundAction
        + RedeemAction
        + RefundAction
        + Clone,
{
    let role = swap.get_role();
    let swap_resource = SwapResource { role: Http(role) };

    let mut entity = siren::Entity::default()
        .with_class_member("swap")
        .with_properties(swap_resource)
        .map_err(|e| {
            tracing::error!("failed to set properties of entity: {:?}", e);
            HttpApiProblem::with_title_and_type_from_status(StatusCode::INTERNAL_SERVER_ERROR)
        })?
        .with_link(siren::NavigationalLink::new(
            &["self"],
            route_factory::swap_path(swap_id),
        ));

    let alpha_params = swap.get_alpha_params();
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

    let beta_params = swap.get_beta_params();
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

    match (swap.get_alpha_events(), swap.get_beta_events()) {
        (Some(alpha_tx), Some(beta_tx)) => {
            let alpha_state_sub = siren::SubEntity::from_entity(
                siren::Entity::default()
                    .with_class_member("state")
                    .with_properties(alpha_tx)
                    .map_err(|e| {
                        tracing::error!("failed to set properties of entity: {:?}", e);
                        HttpApiProblem::with_title_and_type_from_status(
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?,
                &["alpha"],
            );
            entity.push_sub_entity(alpha_state_sub);

            let beta_state_sub = siren::SubEntity::from_entity(
                siren::Entity::default()
                    .with_class_member("state")
                    .with_properties(beta_tx)
                    .map_err(|e| {
                        tracing::error!("failed to set properties of entity: {:?}", e);
                        HttpApiProblem::with_title_and_type_from_status(
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?,
                &["beta"],
            );
            entity.push_sub_entity(beta_state_sub);

            let maybe_action_names = vec![
                swap.init_action().map(|_| "init"),
                swap.deploy_action().map(|_| "deploy"),
                swap.fund_action().map(|_| "fund"),
                swap.redeem_action().map(|_| "redeem"),
                swap.refund_action().map(|_| "refund"),
            ];

            Ok(maybe_action_names
                .into_iter()
                .filter_map(|action| action.ok())
                .fold(entity, |acc, action_name| {
                    let siren_action = make_siren_action(swap_id, action_name);
                    acc.with_action(siren_action)
                }))
        }
        _ => Ok(entity),
    }
}

fn make_siren_action(swap_id: LocalSwapId, action_name: &str) -> siren::Action {
    siren::Action {
        name: action_name.to_owned(),
        class: vec![],
        method: Some(http::Method::GET),
        href: format!("/swaps/{}/{}", swap_id, action_name),
        title: None,
        _type: None,
        fields: vec![],
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
    let swap: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
        facade.load(id).await?;
    let action = swap.init_action()?;
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
    let swap: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
        facade.load(id).await?;
    let action = swap.deploy_action()?;
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
            let swap: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(id).await?;
            let action = swap.fund_action()?;

            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(id).await?;
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
            let swap: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(id).await?;
            let action = swap.redeem_action()?;

            ActionResponseBody::from(action)
        }
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Bob,
        } => {
            let swap: BobSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(id).await?;
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
    match facade.load(id).await? {
        Swap {
            alpha: Protocol::Herc20,
            beta: Protocol::Halight,
            role: Role::Alice,
        } => {
            let swap: AliceSwap<asset::Erc20, asset::Bitcoin, Herc20Finalized, HalightFinalized> =
                facade.load(id).await?;
            let action = swap.refund_action()?;
            let response = ActionResponseBody::from(action);

            Ok(response)
        }
        _ => Err(ActionNotFound.into()),
    }
}
