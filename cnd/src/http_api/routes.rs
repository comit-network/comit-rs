pub mod index;
pub mod peers;
pub mod rfc003;

use crate::{
    asset, htlc_location,
    http_api::{action::ToSirenAction, problem},
    network::comit_ln,
    swap_protocols::{
        actions::{ethereum, lightning, Actions},
        halight::InvoiceState,
        rfc003::LedgerState,
        state::Get,
        Facade2, Role, SwapId,
    },
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
        lightning::AddHoldInvoice,
        ethereum::DeployContract,
        lightning::SettleInvoice,
        ethereum::CallContract,
    >;

    fn actions(&self) -> Vec<Self::ActionKind> {
        if let InvoiceState::None = self.beta_ledger_state {
            return vec![ActionKind::Init(unimplemented!())];
        }

        if let InvoiceState::Added = self.beta_ledger_state {
            return vec![ActionKind::Fund(unimplemented!())];
        }

        let mut actions = vec![];

        if let LedgerState::Funded { .. } = self.alpha_ledger_state {
            actions.push(ActionKind::Refund(unimplemented!()));
        }

        if let InvoiceState::PaymentSent = self.beta_ledger_state {
            actions.push(ActionKind::Redeem(unimplemented!()));
        }

        actions
    }
}

impl Actions for BobEthLnState {
    type ActionKind =
        ActionKind<Never, lightning::SendPayment, ethereum::CallContract, lightning::CancelInvoice>;

    fn actions(&self) -> Vec<Self::ActionKind> {
        let mut actions = vec![];

        if let LedgerState::Funded { .. } = self.alpha_ledger_state {
            if let InvoiceState::Added = self.beta_ledger_state {
                actions.push(ActionKind::Fund(unimplemented!()));
            }
        }

        if let InvoiceState::PaymentSent = self.beta_ledger_state {
            actions.push(ActionKind::Refund(unimplemented!()));
        }

        if let LedgerState::Funded { .. } = self.alpha_ledger_state {
            actions.push(ActionKind::Redeem(unimplemented!()));
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
