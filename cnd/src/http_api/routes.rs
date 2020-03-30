pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset,
    ethereum::{Bytes, U256},
    htlc_location,
    http_api::{action::ToSirenAction, problem},
    network::comit_ln,
    swap_protocols::{
        actions::{
            ethereum,
            lnd::{self, Chain, Network},
            Actions,
        },
        halight::InvoiceState,
        ledger::ethereum::ChainId,
        rfc003::{LedgerState, Secret},
        state::Get,
        Facade2, Role, SwapId,
    },
    timestamp::Timestamp,
    transaction, Never,
};
use http_api_problem::HttpApiProblem;
use warp::{Rejection, Reply};

pub fn into_rejection(problem: HttpApiProblem) -> Rejection {
    warp::reject::custom(problem)
}

// TODO this should accept `NodeLocalSwapId`
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

    let beta_ledger_state: InvoiceState = facade
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
    pub beta_ledger_state: InvoiceState,
    pub finalized_swap: comit_ln::FinalizedSwap,
}

#[derive(Debug)]
pub struct BobEthLnState {
    pub alpha_ledger_state:
        LedgerState<asset::Ether, htlc_location::Ethereum, transaction::Ethereum>,
    pub beta_ledger_state: InvoiceState,
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
        if let InvoiceState::None = self.beta_ledger_state {
            let amount = self.finalized_swap.beta_asset;
            let secret_hash = self.finalized_swap.secret_hash;
            let expiry = unimplemented!();
            let cltv_delta = unimplemented!();
            let chain = Chain::Bitcoin;
            let network = Network::DevNet;
            let self_public_key = self.finalized_swap.alpha_ledger_redeem_identity;

            return vec![ActionKind::Init(lnd::AddHoldInvoice {
                amount,
                secret_hash,
                expiry,
                cltv_delta,
                chain,
                network,
                self_public_key,
            })];
        }

        if let InvoiceState::Added = self.beta_ledger_state {
            let data: Bytes = unimplemented!();
            let amount = self.finalized_swap.alpha_asset;
            let gas_limit: U256 = unimplemented!();
            let chain_id = ChainId::regtest();

            return vec![ActionKind::Fund(ethereum::DeployContract {
                data,
                amount,
                gas_limit,
                chain_id,
            })];
        }

        let mut actions = vec![];

        if let InvoiceState::PaymentSent = self.beta_ledger_state {
            let secret: Secret = unimplemented!();
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

        if let LedgerState::Funded { .. } = self.alpha_ledger_state {
            if let InvoiceState::PaymentSent = self.beta_ledger_state {
                let to = self.finalized_swap.alpha_ledger_refund_identity;
                let data: Option<Bytes> = unimplemented!();
                let gas_limit: U256 = unimplemented!();
                let chain_id = ChainId::regtest();
                let min_block_timestamp: Option<Timestamp> = unimplemented!();

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
    // TODO: Verify and remove this comment; No refund action for Bob, if hold
    // invoice payment expires then the money is never transferred anywhere so no
    // need to refund.
    type ActionKind = ActionKind<Never, lnd::SendPayment, ethereum::CallContract, Never>;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let mut actions = vec![];

        if let LedgerState::Funded { .. } = self.alpha_ledger_state {
            if let InvoiceState::Added = self.beta_ledger_state {
                let to_public_key = self.finalized_swap.beta_ledger_redeem_identity;
                let amount = self.finalized_swap.beta_asset;
                let secret_hash = self.finalized_swap.secret_hash;
                let final_cltv_delta: u32 = unimplemented!();
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
                }))
            }
        }

        if let LedgerState::Funded { .. } = self.alpha_ledger_state {
            let to = self.finalized_swap.alpha_ledger_redeem_identity;
            let data: Option<Bytes> = unimplemented!();
            let gas_limit: U256 = unimplemented!();
            let chain_id: ChainId = ChainId::regtest();
            let min_block_timestamp: Option<Timestamp> = unimplemented!();

            actions.push(ActionKind::Redeem(ethereum::CallContract {
                to,
                data,
                gas_limit,
                chain_id,
                min_block_timestamp,
            }))
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
