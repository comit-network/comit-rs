pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset,
    ethereum::Bytes,
    htlc_location,
    http_api::{action::ActionResponseBody, problem},
    network::comit_ln,
    swap_protocols::{
        actions::{
            ethereum,
            lnd::{self, Chain},
        },
        halight::{self, data},
        ledger::ethereum::ChainId,
        rfc003::LedgerState,
        state::Get,
        Facade2, FundAction, InitAction, NodeLocalSwapId, RedeemAction, RefundAction, Role, SwapId,
    },
    transaction,
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use http_api_problem::HttpApiProblem;
use warp::{http, Rejection, Reply};

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(problem)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn get_han_halight_swap(
    id: NodeLocalSwapId,
    facade: Facade2,
) -> Result<impl Reply, Rejection> {
    handle_get_han_halight_swap(facade, id)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

pub async fn handle_get_han_halight_swap(
    facade: Facade2,
    local_id: NodeLocalSwapId,
) -> anyhow::Result<siren::Entity> {
    let swap_id = SwapId(local_id.0); // FIXME: Resolve this abuse.

    // This is ok, we use a new create_watcher in han.rs and call it with local id.
    let alpha_ledger_state: Option<
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    > = facade.alpha_ledger_state.get(&swap_id).await?;

    // And again here, we munge the swap_id when calling create_watcher.
    let beta_ledger_state = facade.beta_ledger_state.get(&swap_id).await?;

    let finalized_swap = facade.get_finalized_swap(local_id).await;

    let (alpha_ledger_state, beta_ledger_state, finalized_swap) =
        match (alpha_ledger_state, beta_ledger_state, finalized_swap) {
            (Some(alpha_ledger_state), Some(beta_ledger_state), Some(finalized_swap)) => {
                (alpha_ledger_state, beta_ledger_state, finalized_swap)
            }
            _ => {
                let empty_swap = make_swap_entity(swap_id, vec![]);

                tracing::debug!(
                    "returning empty siren document because states are not yet completed"
                );

                return Ok(empty_swap);
            }
        };

    let entity = match finalized_swap.role {
        Role::Alice => {
            let state = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            let maybe_action_names = vec![
                state.init_action().map(|_| "init"),
                state.fund_action().map(|_| "fund"),
                state.redeem_action().map(|_| "redeem"),
                state.refund_action().map(|_| "refund"),
            ];

            make_swap_entity(swap_id, maybe_action_names)
        }
        Role::Bob => {
            let state = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            // Bob cannot init and refund in this swap combination
            let maybe_action_names = vec![
                state.fund_action().map(|_| "fund"),
                state.redeem_action().map(|_| "redeem"),
            ];

            make_swap_entity(swap_id, maybe_action_names)
        }
    };

    Ok(entity)
}

fn make_swap_entity(swap_id: SwapId, maybe_action_names: Vec<Option<&str>>) -> siren::Entity {
    let swap = siren::Entity::default().with_class_member("swap");

    maybe_action_names
        .into_iter()
        .filter_map(|action| action)
        .fold(swap, |acc, action_name| {
            let siren_action = make_siren_action(swap_id, action_name);

            acc.with_action(siren_action)
        })
}

fn make_siren_action(swap_id: SwapId, action_name: &str) -> siren::Action {
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

#[derive(Debug)]
pub struct AliceEthLnState {
    pub alpha_ledger_state:
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    pub beta_ledger_state: halight::State,
    pub finalized_swap: comit_ln::FinalizedSwap,
}

#[derive(Debug)]
pub struct BobEthLnState {
    pub alpha_ledger_state:
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    pub beta_ledger_state: halight::State,
    pub finalized_swap: comit_ln::FinalizedSwap,
}

impl InitAction for AliceEthLnState {
    type Output = lnd::AddHoldInvoice;

    fn init_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Unknown => {
                let amount = self.finalized_swap.beta_asset;
                let secret_hash = self.finalized_swap.secret_hash;
                let expiry = 3600;
                let cltv_expiry = self.finalized_swap.beta_expiry.into();
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_redeem_identity;

                Some(lnd::AddHoldInvoice {
                    amount,
                    secret_hash,
                    expiry,
                    cltv_expiry,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => None,
        }
    }
}

impl FundAction for AliceEthLnState {
    type Output = ethereum::DeployContract;

    fn fund_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Opened(_) => {
                let eth_htlc = self.finalized_swap.han_params();
                let data = eth_htlc.into();
                let amount = self.finalized_swap.alpha_asset.clone();
                let gas_limit = EtherHtlc::deploy_tx_gas_limit();
                let chain_id = ChainId::regtest();

                Some(ethereum::DeployContract {
                    data,
                    amount,
                    gas_limit,
                    chain_id,
                })
            }
            _ => None,
        }
    }
}

impl RedeemAction for AliceEthLnState {
    type Output = lnd::SettleInvoice;

    fn redeem_action(&self) -> Option<Self::Output> {
        match self.beta_ledger_state {
            halight::State::Accepted(_) => {
                let secret = self.finalized_swap.secret.unwrap(); // unwrap ok since only Alice calls this.
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_redeem_identity;

                Some(lnd::SettleInvoice {
                    secret,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => None,
        }
    }
}

impl RefundAction for AliceEthLnState {
    type Output = ethereum::CallContract;

    fn refund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (LedgerState::Funded { htlc_location, .. }, halight::State::Accepted(_)) => {
                let to = *htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = Some(self.finalized_swap.alpha_expiry);

                Some(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => None,
        }
    }
}

impl FundAction for BobEthLnState {
    type Output = lnd::SendPayment;

    fn fund_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (LedgerState::Funded { .. }, halight::State::Opened(_)) => {
                let to_public_key = self.finalized_swap.beta_ledger_redeem_identity;
                let amount = self.finalized_swap.beta_asset;
                let secret_hash = self.finalized_swap.secret_hash;
                let final_cltv_delta = self.finalized_swap.beta_expiry.into();
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_refund_identity;

                Some(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                    self_public_key,
                })
            }
            _ => None,
        }
    }
}

impl RedeemAction for BobEthLnState {
    type Output = ethereum::CallContract;

    fn redeem_action(&self) -> Option<Self::Output> {
        match (&self.alpha_ledger_state, &self.beta_ledger_state) {
            (
                LedgerState::Funded { htlc_location, .. },
                halight::State::Settled(data::Settled { secret }),
            ) => {
                let to = *htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let chain_id: ChainId = ChainId::regtest();
                let min_block_timestamp = None;

                Some(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                })
            }
            _ => None,
        }
    }
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_init(id: NodeLocalSwapId, facade: Facade2) -> Result<impl Reply, Rejection> {
    handle_action_init(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_init(
    local_id: NodeLocalSwapId,
    facade: Facade2,
) -> anyhow::Result<ActionResponseBody> {
    let id = SwapId(local_id.0);

    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", id))?;

    let finalized_swap = facade
        .get_finalized_swap(local_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.init_action().map(ActionResponseBody::from)
        }
        Role::Bob => None,
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_fund(id: NodeLocalSwapId, facade: Facade2) -> Result<impl Reply, Rejection> {
    handle_action_fund(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_fund(
    local_id: NodeLocalSwapId,
    facade: Facade2,
) -> anyhow::Result<ActionResponseBody> {
    let id = SwapId(local_id.0);
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", id))?;

    let finalized_swap = facade
        .get_finalized_swap(local_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.fund_action().map(ActionResponseBody::from)
        }
        Role::Bob => {
            let state = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.fund_action().map(ActionResponseBody::from)
        }
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_redeem(id: NodeLocalSwapId, facade: Facade2) -> Result<impl Reply, Rejection> {
    handle_action_redeem(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_redeem(
    local_id: NodeLocalSwapId,
    facade: Facade2,
) -> anyhow::Result<ActionResponseBody> {
    let id = SwapId(local_id.0);
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", id))?;

    let finalized_swap = facade
        .get_finalized_swap(local_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.redeem_action().map(ActionResponseBody::from)
        }
        Role::Bob => {
            let state = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.redeem_action().map(ActionResponseBody::from)
        }
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
}

#[allow(clippy::needless_pass_by_value)]
pub async fn action_refund(id: NodeLocalSwapId, facade: Facade2) -> Result<impl Reply, Rejection> {
    handle_action_refund(id, facade)
        .await
        .map(|body| warp::reply::json(&body))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

#[allow(clippy::unit_arg, clippy::let_unit_value, clippy::cognitive_complexity)]
async fn handle_action_refund(
    local_id: NodeLocalSwapId,
    facade: Facade2,
) -> anyhow::Result<ActionResponseBody> {
    let id = SwapId(local_id.0);
    let alpha_ledger_state: LedgerState<
        asset::Ether,
        htlc_location::Ethereum,
        transaction::Ethereum,
    > = facade
        .alpha_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("alpha ledger state not found for {}", id))?;

    let beta_ledger_state: halight::State = facade
        .beta_ledger_state
        .get(&id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("beta ledger state not found for {}", id))?;

    let finalized_swap = facade
        .get_finalized_swap(local_id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", id))?;

    let maybe_response = match finalized_swap.role {
        Role::Alice => {
            let state = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            };

            state.refund_action().map(ActionResponseBody::from)
        }
        _ => None,
    };

    let response = maybe_response.ok_or(LndActionError::NotFound)?;

    Ok(response)
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum LndActionError {
    #[error("action not found")]
    NotFound,
}
