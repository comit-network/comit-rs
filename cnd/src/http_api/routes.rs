pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset,
    ethereum::Bytes,
    htlc_location,
    http_api::{
        action::{ActionResponseBody, ToSirenAction},
        problem, Http,
    },
    network::comit_ln,
    swap_protocols::{
        actions::{
            ethereum,
            lnd::{self, Chain},
            Actions,
        },
        halight::{self, data},
        ledger::ethereum::ChainId,
        rfc003::LedgerState,
        state::Get,
        Facade2, NodeLocalSwapId, Role, SwapId,
    },
    transaction, Never,
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
                // TODO: for now we just default to an empty swap,
                // This means any ID thrown at this function will yield a 200 - that is not
                // desireable Once we have the database, we can actually check
                // whether we have a swap with this ID available and get decide between a 404
                // and an empty swap without actions
                let empty_swap = siren::Entity::default().with_class_member("swap");

                tracing::debug!(
                    "returning empty siren document because states are not yet completed"
                );

                return Ok(empty_swap);
            }
        };

    let entity = match finalized_swap.role {
        Role::Alice => {
            let actions = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            make_entity(actions, swap_id)
        }
        Role::Bob => {
            let actions = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            make_entity(actions, swap_id)
        }
    };

    Ok(entity)
}

fn make_entity<A: ToSirenAction>(actions: Vec<A>, id: SwapId) -> siren::Entity {
    let entity = siren::Entity::default().with_class_member("swap");

    actions.into_iter().fold(entity, |acc, action| {
        let action = action.to_siren_action(&id);
        acc.with_action(action)
    })
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

// TODO this should be in COMIT library that doesn't exist yet
impl Actions for AliceEthLnState {
    type ActionKind = ActionKind<
        lnd::AddHoldInvoice,
        ethereum::DeployContract,
        lnd::SettleInvoice,
        ethereum::CallContract,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        if let halight::State::Unknown = self.beta_ledger_state {
            let amount = self.finalized_swap.beta_asset;
            let secret_hash = self.finalized_swap.secret_hash;
            let expiry = 3600; // TODO: don't hardcode this
            let cltv_expiry = self.finalized_swap.beta_expiry.into();
            let chain = Chain::Bitcoin;
            let network = bitcoin::Network::Regtest;
            let self_public_key = self.finalized_swap.beta_ledger_redeem_identity;

            return vec![ActionKind::Init(lnd::AddHoldInvoice {
                amount,
                secret_hash,
                expiry,
                cltv_expiry,
                chain,
                network,
                self_public_key,
            })];
        }

        if let halight::State::Opened(_) = self.beta_ledger_state {
            let eth_htlc = self.finalized_swap.han_params();
            let data = eth_htlc.into();
            let amount = self.finalized_swap.alpha_asset.clone();
            let gas_limit = EtherHtlc::deploy_tx_gas_limit();
            let chain_id = ChainId::regtest();

            return vec![ActionKind::Fund(ethereum::DeployContract {
                data,
                amount,
                gas_limit,
                chain_id,
            })];
        }

        let mut actions = vec![];

        if let halight::State::Accepted(_) = self.beta_ledger_state {
            let secret = self.finalized_swap.secret.unwrap(); // unwrap ok since only Alice calls this.
            let chain = Chain::Bitcoin;
            let network = bitcoin::Network::Regtest;
            let self_public_key = self.finalized_swap.beta_ledger_redeem_identity;

            actions.push(ActionKind::Redeem(lnd::SettleInvoice {
                secret,
                chain,
                network,
                self_public_key,
            }))
        }

        if let LedgerState::Funded { htlc_location, .. } = self.alpha_ledger_state {
            if let halight::State::Accepted(_) = self.beta_ledger_state {
                let to = htlc_location;
                let data = None;
                let gas_limit = EtherHtlc::refund_tx_gas_limit();
                let chain_id = ChainId::regtest();
                let min_block_timestamp = Some(self.finalized_swap.alpha_expiry);

                actions.push(ActionKind::Refund(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                }));
            }
        }
        actions
    }
}

impl Actions for BobEthLnState {
    type ActionKind = ActionKind<Never, lnd::SendPayment, ethereum::CallContract, Never>;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let mut actions = vec![];

        if let LedgerState::Funded { htlc_location, .. } = self.alpha_ledger_state {
            if let halight::State::Opened(_) = self.beta_ledger_state {
                let to_public_key = self.finalized_swap.beta_ledger_redeem_identity;
                let amount = self.finalized_swap.beta_asset;
                let secret_hash = self.finalized_swap.secret_hash;
                let final_cltv_delta = self.finalized_swap.beta_expiry.into();
                let chain = Chain::Bitcoin;
                let network = bitcoin::Network::Regtest;
                let self_public_key = self.finalized_swap.beta_ledger_refund_identity;

                actions.push(ActionKind::Fund(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    final_cltv_delta,
                    chain,
                    network,
                    self_public_key,
                }));
            }

            if let halight::State::Settled(data::Settled { secret }) = self.beta_ledger_state {
                let to = htlc_location;
                let data = Some(Bytes::from(secret.into_raw_secret().to_vec()));
                let gas_limit = EtherHtlc::redeem_tx_gas_limit();
                let chain_id: ChainId = ChainId::regtest();
                let min_block_timestamp = None;

                actions.push(ActionKind::Redeem(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                }))
            }
        }
        actions
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ActionKind<TInit, TFund, TRedeem, TRefund> {
    Init(TInit),
    Fund(TFund),
    Redeem(TRedeem),
    Refund(TRefund),
}

// all our actions for this particular case don't have any parameters, so we can
// just implement this generically
impl<TInit, TFund, TRedeem, TRefund> ToSirenAction for ActionKind<TInit, TFund, TRedeem, TRefund> {
    // FIXME: for han-halight this is the node local swap id
    fn to_siren_action(&self, id: &SwapId) -> siren::Action {
        let name = match self {
            ActionKind::Init(_) => "init",
            ActionKind::Fund(_) => "fund",
            ActionKind::Redeem(_) => "redeem",
            ActionKind::Refund(_) => "refund",
        };

        siren::Action {
            name: name.to_owned(),
            class: vec![],
            method: Some(http::Method::GET),
            href: format!("/swaps/{}/{}", id, name),
            title: None,
            _type: None,
            fields: vec![],
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
    let id = SwapId(local_id.0); // FIXME: The insert/get/update traits use a SwapId

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

    if let Role::Alice = finalized_swap.role {
        let actions = AliceEthLnState {
            alpha_ledger_state,
            beta_ledger_state,
            finalized_swap,
        }
        .actions();

        for action in actions {
            if let ActionKind::Init(lnd::AddHoldInvoice {
                amount,
                secret_hash,
                expiry,
                cltv_expiry,
                chain,
                network,
                self_public_key,
            }) = action
            {
                return Ok(ActionResponseBody::LndAddHoldInvoice {
                    amount: Http(amount),
                    secret_hash,
                    expiry,
                    cltv_expiry,
                    chain: Http(chain),
                    network: Http(network),
                    self_public_key,
                });
            }
        }
    };
    Err(LndActionError::NotFound.into())
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
    let id = SwapId(local_id.0); // FIXME: The insert/get/update traits use a SwapId
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

    match finalized_swap.role {
        Role::Alice => {
            let actions = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            for action in actions {
                if let ActionKind::Fund(ethereum::DeployContract {
                    amount,
                    chain_id,
                    gas_limit,
                    data,
                }) = action
                {
                    return Ok(ActionResponseBody::EthereumDeployContract {
                        data,
                        amount,
                        gas_limit: gas_limit.into(),
                        chain_id,
                    });
                }
            }
        }
        Role::Bob => {
            let actions = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            for action in actions {
                if let ActionKind::Fund(lnd::SendPayment {
                    to_public_key,
                    amount,
                    secret_hash,
                    network,
                    chain,
                    final_cltv_delta,
                    self_public_key,
                }) = action
                {
                    return Ok(ActionResponseBody::LndSendPayment {
                        to_public_key,
                        amount: amount.into(),
                        secret_hash,
                        network: network.into(),
                        chain: chain.into(),
                        final_cltv_delta,
                        self_public_key,
                    });
                }
            }
        }
    }
    Err(LndActionError::NotFound.into())
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
    let id = SwapId(local_id.0); // FIXME: The insert/get/update traits use a SwapId
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

    match finalized_swap.role {
        Role::Alice => {
            let actions = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            for action in actions {
                if let ActionKind::Redeem(lnd::SettleInvoice {
                    secret,
                    chain,
                    network,
                    self_public_key,
                }) = action
                {
                    return Ok(ActionResponseBody::LndSettleInvoice {
                        secret,
                        chain: chain.into(),
                        network: network.into(),
                        self_public_key,
                    });
                }
            }
        }
        Role::Bob => {
            let actions = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            for action in actions {
                if let ActionKind::Redeem(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                }) = action
                {
                    return Ok(ActionResponseBody::EthereumCallContract {
                        contract_address: to,
                        data,
                        gas_limit: gas_limit.into(),
                        chain_id,
                        min_block_timestamp,
                    });
                }
            }
        }
    }
    Err(LndActionError::NotFound.into())
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
    let id = SwapId(local_id.0); // FIXME: The insert/get/update traits use a SwapId
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

    match finalized_swap.role {
        Role::Alice => {
            let actions = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            for action in actions {
                if let ActionKind::Refund(ethereum::CallContract {
                    to,
                    data,
                    gas_limit,
                    chain_id,
                    min_block_timestamp,
                }) = action
                {
                    return Ok(ActionResponseBody::EthereumCallContract {
                        contract_address: to,
                        data,
                        gas_limit: gas_limit.into(),
                        chain_id,
                        min_block_timestamp,
                    });
                }
            }
        }
        Role::Bob => {
            // There is no refund action for Bob when he is the HALight sender.
        }
    }
    Err(LndActionError::NotFound.into())
}

#[derive(Debug, Clone, Copy, thiserror::Error)]
pub enum LndActionError {
    #[error("action not found")]
    NotFound,
}
