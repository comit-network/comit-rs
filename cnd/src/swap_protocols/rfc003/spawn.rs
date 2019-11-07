use crate::{
    dependencies::Dependencies,
    swap_protocols::{
        asset::Asset,
        rfc003::{
            messages::AcceptResponseBody,
            state_machine::{self, SwapStates},
            CreateLedgerEvents, Ledger, Request,
        },
        LedgerConnectors,
    },
};
use futures::{sync::mpsc, Future};
use std::sync::Arc;

pub trait Spawn: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: Request<AL, BL, AA, BA>,
        accept: AcceptResponseBody<AL, BL>,
    ) -> mpsc::UnboundedReceiver<SwapStates<AL, BL, AA, BA>>
    where
        LedgerConnectors: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

impl Spawn for Dependencies {
    #[allow(clippy::type_complexity)]
    fn spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        request: Request<AL, BL, AA, BA>,
        accept: AcceptResponseBody<AL, BL>,
    ) -> mpsc::UnboundedReceiver<SwapStates<AL, BL, AA, BA>>
    where
        LedgerConnectors: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
    {
        let id = request.id;

        let (sender, receiver) = mpsc::unbounded();

        let context = state_machine::Context {
            alpha_ledger_events: self.ledger_events.create_ledger_events(),
            beta_ledger_events: self.ledger_events.create_ledger_events(),
            state_repo: Arc::new(sender),
        };

        let swap_execution = state_machine::Swap::start_in(
            state_machine::Start {
                swap: state_machine::OngoingSwap {
                    alpha_ledger: request.alpha_ledger,
                    beta_ledger: request.beta_ledger,
                    alpha_asset: request.alpha_asset,
                    beta_asset: request.beta_asset,
                    hash_function: request.hash_function,
                    alpha_ledger_redeem_identity: accept.alpha_ledger_redeem_identity,
                    alpha_ledger_refund_identity: request.alpha_ledger_refund_identity,
                    beta_ledger_redeem_identity: request.beta_ledger_redeem_identity,
                    beta_ledger_refund_identity: accept.beta_ledger_refund_identity,
                    alpha_expiry: request.alpha_expiry,
                    beta_expiry: request.beta_expiry,
                    secret_hash: request.secret_hash,
                },
            },
            context,
        )
        .map(move |outcome| log::info!("Swap {} finished with {:?}", id, outcome))
        .map_err(move |e| log::error!("Swap {} failed with {:?}", id, e));

        tokio::spawn(swap_execution);

        receiver
    }
}
