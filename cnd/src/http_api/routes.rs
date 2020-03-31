pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset,
    ethereum::Bytes,
    htlc_location,
    http_api::{action::ToSirenAction, problem},
    network::comit_ln,
    swap_protocols::{
        actions::{
            ethereum,
            lnd::{self, Chain, Network},
            Actions,
        },
        halight::{self, data},
        ledger::ethereum::ChainId,
        rfc003::LedgerState,
        state::Get,
        Facade2, Role, SwapId,
    },
    transaction, Never,
};
use blockchain_contracts::ethereum::rfc003::ether_htlc::EtherHtlc;
use http_api_problem::HttpApiProblem;
use warp::{Rejection, Reply};

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(problem)
}

// TODO: this should accept `NodeLocalSwapId`
// This will be possible once the `swap_protocol::LedgerState` struct
// is duplicated in `han::LedgerState` and `herc20::LedgerState`
#[allow(clippy::needless_pass_by_value)]
pub async fn get_han_halight_swap(id: SwapId, facade: Facade2) -> Result<impl Reply, Rejection> {
    handle_get_han_halight_swap(facade, id)
        .await
        .map(|swap_resource| warp::reply::json(&swap_resource))
        .map_err(problem::from_anyhow)
        .map_err(into_rejection)
}

pub async fn handle_get_han_halight_swap(
    facade: Facade2,
    id: SwapId,
) -> anyhow::Result<siren::Entity> {
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
        .get_finalized_swap(id)
        .await
        .ok_or_else(|| anyhow::anyhow!("swap with id {} not found", id))?;

    let entity = match finalized_swap.role {
        Role::Alice => {
            let actions = AliceEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            make_entity(actions, id)
        }
        Role::Bob => {
            let actions = BobEthLnState {
                alpha_ledger_state,
                beta_ledger_state,
                finalized_swap,
            }
            .actions();

            make_entity(actions, id)
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
            let expiry = self.finalized_swap.alpha_expiry.into(); // Lazy choice, if Bob has not funded by this time Alice will refund anyways.
            let cltv_expiry = self.finalized_swap.beta_expiry.into();
            let chain = Chain::Bitcoin;
            let network = Network::DevNet;
            let self_public_key = self.finalized_swap.beta_ledger_refund_identity;

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
            let network = Network::DevNet;
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
                let network = Network::DevNet;
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

impl<TInit, TFund, TRedeem, TRefund> ToSirenAction for ActionKind<TInit, TFund, TRedeem, TRefund> {
    fn to_siren_action(&self, _id: &SwapId) -> siren::Action {
        unimplemented!()
    }
}
