pub mod actions;

use crate::{
    asset::Asset,
    swap_protocols::rfc003::{
        self, ledger::Ledger, ledger_state::LedgerState, messages::Request, Accept, ActorState,
        Decline, DeriveIdentities, SwapCommunication,
    },
};
use derivative::Derivative;
use futures::sync::oneshot;
use std::sync::{Arc, Mutex};

#[allow(type_alias_bounds)]
pub type ResponseSender<AL, BL>
where
    AL: Ledger,
    BL: Ledger,
= Arc<Mutex<Option<oneshot::Sender<rfc003::Response<AL, BL>>>>>;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct State<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    pub swap_communication: SwapCommunication<AL, BL, AA, BA>,
    pub alpha_ledger_state: LedgerState<AL::HtlcLocation, AL::Transaction, AA>,
    pub beta_ledger_state: LedgerState<BL::HtlcLocation, BL::Transaction, BA>,
    #[derivative(Debug = "ignore")]
    pub secret_source: Arc<dyn DeriveIdentities>,
    pub failed: bool, // Gets set on any error during the execution of a swap.
}

impl<AL, BL, AA, BA> State<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    pub fn proposed(
        request: Request<AL, BL, AA, BA>,
        secret_source: impl DeriveIdentities,
    ) -> Self {
        Self {
            swap_communication: SwapCommunication::Proposed { request },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source: Arc::new(secret_source),
            failed: false,
        }
    }

    pub fn accepted(
        request: Request<AL, BL, AA, BA>,
        response: Accept<AL::Identity, BL::Identity>,
        secret_source: impl DeriveIdentities,
    ) -> Self {
        Self {
            swap_communication: SwapCommunication::Accepted { request, response },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source: Arc::new(secret_source),
            failed: false,
        }
    }

    pub fn declined(
        request: Request<AL, BL, AA, BA>,
        response: Decline,
        secret_source: impl DeriveIdentities,
    ) -> Self {
        Self {
            swap_communication: SwapCommunication::Declined { request, response },
            alpha_ledger_state: LedgerState::NotDeployed,
            beta_ledger_state: LedgerState::NotDeployed,
            secret_source: Arc::new(secret_source),
            failed: false,
        }
    }

    pub fn request(&self) -> &Request<AL, BL, AA, BA> {
        match &self.swap_communication {
            SwapCommunication::Accepted { request, .. }
            | SwapCommunication::Proposed { request, .. }
            | SwapCommunication::Declined { request, .. } => request,
        }
    }
}

impl<AL, BL, AA, BA> ActorState for State<AL, BL, AA, BA>
where
    AL: Ledger,
    BL: Ledger,
    AA: Asset,
    BA: Asset,
{
    type AL = AL;
    type BL = BL;
    type AA = AA;
    type BA = BA;

    fn expected_alpha_asset(&self) -> &Self::AA {
        &self.swap_communication.request().alpha_asset
    }

    fn expected_beta_asset(&self) -> &Self::BA {
        &self.swap_communication.request().beta_asset
    }

    fn alpha_ledger_mut(&mut self) -> &mut LedgerState<AL::HtlcLocation, AL::Transaction, AA> {
        &mut self.alpha_ledger_state
    }

    fn beta_ledger_mut(&mut self) -> &mut LedgerState<BL::HtlcLocation, BL::Transaction, BA> {
        &mut self.beta_ledger_state
    }

    fn swap_failed(&self) -> bool {
        self.failed
    }

    fn set_swap_failed(&mut self) {
        self.failed = true;
    }
}
