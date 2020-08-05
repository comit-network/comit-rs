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
            ActionName, AlphaAbsoluteExpiry, AlphaEvents, AlphaLedger, AlphaParams,
            BetaAbsoluteExpiry, BetaEvents, BetaLedger, BetaParams, GetRole, Ledger, LedgerEvents,
        },
        route_factory, Http,
    },
    storage::Load,
    DeployAction, Facade, FundAction, InitAction, LocalSwapId, RedeemAction, RefundAction, Role,
};
use comit::rsa::SwapTime;
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
        if is_safe_to_init(swap) {
            return Ok(Some(ActionName::Init));
        }
        return Ok(None);
    }

    if swap.deploy_action().is_ok() {
        if is_safe_to_deploy(swap) {
            return Ok(Some(ActionName::Deploy));
        }
        return Ok(None);
    }

    if swap.fund_action().is_ok() {
        if is_safe_to_fund(swap) {
            return Ok(Some(ActionName::Fund));
        }
        return Ok(None);
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
        if is_safe_to_redeem(swap) {
            return Ok(Some(ActionName::Redeem));
        }
        return Ok(Some(ActionName::Refund));
    }

    Ok(None)
}

fn is_safe_to_init<S>(swap: &S) -> bool
where
    S: GetRole + AlphaLedger + BetaLedger + AlphaAbsoluteExpiry + BetaAbsoluteExpiry,
{
    let rsa = match swap_time(swap) {
        Some(rsa) => rsa,
        None => return false,
    };
    match swap.get_role() {
        Role::Alice => rsa.is_safe_for_alice_to_init(),
        Role::Bob => rsa.is_safe_for_bob_to_init(),
    }
}

fn is_safe_to_deploy<S>(swap: &S) -> bool
where
    S: GetRole + AlphaLedger + BetaLedger + AlphaAbsoluteExpiry + BetaAbsoluteExpiry,
{
    let rsa = match swap_time(swap) {
        Some(rsa) => rsa,
        None => return false,
    };
    match swap.get_role() {
        Role::Alice => rsa.is_safe_for_alice_to_deploy(),
        Role::Bob => rsa.is_safe_for_bob_to_deploy(),
    }
}

fn is_safe_to_fund<S>(swap: &S) -> bool
where
    S: GetRole + AlphaLedger + BetaLedger + AlphaAbsoluteExpiry + BetaAbsoluteExpiry,
{
    let rsa = match swap_time(swap) {
        Some(rsa) => rsa,
        None => return false,
    };
    match swap.get_role() {
        Role::Alice => rsa.is_safe_for_alice_to_fund(),
        Role::Bob => rsa.is_safe_for_bob_to_fund(),
    }
}

fn is_safe_to_redeem<S>(swap: &S) -> bool
where
    S: GetRole + AlphaLedger + BetaLedger + AlphaAbsoluteExpiry + BetaAbsoluteExpiry,
{
    let rsa = match swap_time(swap) {
        Some(rsa) => rsa,
        None => return false,
    };
    match swap.get_role() {
        Role::Alice => rsa.is_safe_for_alice_to_redeem(),
        Role::Bob => rsa.is_safe_for_bob_to_redeem(),
    }
}

fn swap_time<S>(swap: &S) -> Option<SwapTime>
where
    S: GetRole + AlphaLedger + BetaLedger + AlphaAbsoluteExpiry + BetaAbsoluteExpiry,
{
    let alpha_ledger = swap.alpha_ledger();
    let beta_ledger = swap.beta_ledger();
    let (alpha_expiry, beta_expiry) = {
        match (swap.alpha_absolute_expiry(), swap.beta_absolute_expiry()) {
            (Some(alpha), Some(beta)) => (alpha, beta),
            _ => return None,
        }
    };

    let rsa = SwapTime::new(
        alpha_ledger.into(),
        beta_ledger.into(),
        alpha_expiry,
        beta_expiry,
    );

    Some(rsa)
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
