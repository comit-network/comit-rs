use crate::{
    connector::Connector,
    swap_protocols::{
        asset::Asset,
        dependencies::LedgerEventDependencies,
        rfc003::{
            self,
            alice::{self, State, SwapCommunication},
            state_machine,
            state_store::StateStore,
            CreateLedgerEvents, Ledger,
        },
    },
};
use futures::{sync::mpsc, Stream};
use futures_core::{
    compat::Future01CompatExt,
    future::{FutureExt, TryFutureExt},
};
use std::sync::Arc;

pub trait AliceSpawn: Send + Sync + 'static {
    #[allow(clippy::type_complexity)]
    fn alice_spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
        response: rfc003::Response<AL, BL>,
    ) where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>;
}

impl<S> AliceSpawn for Connector<S>
where
    S: Send + Sync + 'static,
{
    #[allow(clippy::type_complexity)]
    fn alice_spawn<AL: Ledger, BL: Ledger, AA: Asset, BA: Asset>(
        &self,
        swap_request: rfc003::Request<AL, BL, AA, BA>,
        response: rfc003::Response<AL, BL>,
    ) where
        LedgerEventDependencies: CreateLedgerEvents<AL, AA> + CreateLedgerEvents<BL, BA>,
        S: Send + Sync + 'static,
    {
        let id = swap_request.id;
        let swap_seed = self.deps.seed.swap_seed(id);

        let (sender, receiver) = mpsc::unbounded();

        let swap_execution = {
            let ledger_events = self.deps.ledger_events.clone();

            async move {
                let alice_state = match response {
                    Ok(accepted) => State::accepted(swap_request, accepted, swap_seed),
                    Err(declined) => State::declined(swap_request, declined, swap_seed),
                };

                match alice_state {
                    State {
                        swap_communication: SwapCommunication::Accepted { request, response },
                        ..
                    } => {
                        let context = state_machine::Context {
                            alpha_ledger_events: ledger_events.create_ledger_events(),
                            beta_ledger_events: ledger_events.create_ledger_events(),
                            state_repo: Arc::new(sender),
                        };

                        let result = state_machine::Swap::start_in(
                            state_machine::Start {
                                swap: state_machine::OngoingSwap {
                                    alpha_ledger: request.alpha_ledger,
                                    beta_ledger: request.beta_ledger,
                                    alpha_asset: request.alpha_asset,
                                    beta_asset: request.beta_asset,
                                    hash_function: request.hash_function,
                                    alpha_ledger_redeem_identity: response
                                        .alpha_ledger_redeem_identity,
                                    alpha_ledger_refund_identity: request
                                        .alpha_ledger_refund_identity,
                                    beta_ledger_redeem_identity: request
                                        .beta_ledger_redeem_identity,
                                    beta_ledger_refund_identity: response
                                        .beta_ledger_refund_identity,
                                    alpha_expiry: request.alpha_expiry,
                                    beta_expiry: request.beta_expiry,
                                    secret_hash: request.secret_hash,
                                },
                            },
                            context,
                        )
                        .compat()
                        .await;

                        match result {
                            Ok(outcome) => log::info!("Swap {} finished with {:?}", id, outcome),
                            Err(e) => log::error!("Swap {} failed with {:?}", id, e),
                        }
                    }
                    _ => {
                        log::info!("Swap {} was declined", id);
                    }
                }

                Ok(())
            }
        };

        let state_store = Arc::clone(&self.deps.state_store);
        tokio::spawn(receiver.for_each(move |update| {
            state_store.update::<alice::State<AL, BL, AA, BA>>(&id, update);
            Ok(())
        }));

        tokio::spawn(swap_execution.boxed().compat());
    }
}
